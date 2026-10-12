//! POSIX threads on `std`'s threads (milestone 836 (a C library, stage 2: threads)).
//!
//! relibc's `pthread` module is built on its own dynamic linker's thread control block and a
//! `clone` system call, neither of which is seeded. nife's C library is built on nife's Rust `std`
//! (`c_library/README.md`), and `std::thread::spawn` already runs real threads in one address space
//! (milestone 812 (`std::thread::spawn` runs real threads in one address space)). So a
//! `pthread_create` is a `std` spawn, and this module is the bookkeeping POSIX asks for on top:
//! a joinable or detached thread, its return value, `pthread_exit`, `pthread_self` and a thread
//! number. relibc's `header/pthread` (the `pthread_*` functions themselves, attributes, mutexes,
//! condition variables, read-write locks, barriers, spin locks, keys) is seeded and calls the
//! functions here by the names relibc's own module gives them.
//!
//! # `pthread_exit`
//!
//! A thread made here starts in `thread_start.c`, which records a `setjmp` before calling the
//! program's start routine. `pthread_exit` runs the cleanup handlers and `longjmp`s back to it,
//! so the thread returns through `std`'s spawn closure like any other, and `std` ends it and frees
//! its stack. The frames skipped are the C program's and `pthread_exit`'s own, which holds nothing
//! with a destructor when it jumps. A `pthread_exit` from the main thread waits for every other
//! thread made here to finish and then ends the process with status 0, which is what a process
//! whose last thread has exited does.
//!
//! # BUGS
//!
//! - **Cancellation is deferred and checked only at `pthread_testcancel`.** nife has no signals to
//!   interrupt a thread, so the cancellation points POSIX lists (a blocking `read`, a condition
//!   wait) do not act on a pending cancel.
//! - **A caller-supplied stack (`pthread_attr_setstack`) is refused** with `EINVAL`: `std` makes
//!   the stack. A requested size is honoured, and `std` raises one below 64 KiB to 64 KiB.
//! - **`pthread_getattr_np` reports no stack address**, for the same reason.
//! - **16 threads a process**, the main thread included: `std`'s limit (milestone 812's `BUGS`).
//!   One more `pthread_create` answers `EAGAIN`.
//! - **Scheduling parameters are accepted and ignored**, since nife's scheduler has one class
//!   (`header/sched`).

use core::cell::{Cell, UnsafeCell};
use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::thread::JoinHandle;

use alloc::boxed::Box;

use crate::error::Errno;
use crate::header::errno::{EAGAIN, EDEADLK, EINVAL, ENOENT, ENOSYS};
use crate::header::pthread as header;
use crate::header::sched::sched_param;
use crate::platform::types::{c_int, c_void, clockid_t, pthread_t};

/// The bits relibc keeps in [`Pthread::flags`]; only `DETACHED` exists.
pub struct PthreadFlags(usize);

impl PthreadFlags {
    pub const DETACHED: Self = Self(1);

    pub const fn bits(&self) -> usize {
        self.0
    }
}

/// Where a thread made here is in its life. Exactly one of its joiner, its detacher and the
/// thread itself frees its [`Pthread`], and these say which.
const RUNNING: u32 = 0;
/// `pthread_detach` came first: the thread frees its own record when it ends.
const DETACHED: u32 = 1;
/// The thread has ended: a later `pthread_detach`, or the joiner, frees the record.
const EXITED: u32 = 2;

/// A `setjmp` buffer big enough for all three architectures (`include/setjmp.h`: 22 words on
/// aarch64, 26 on riscv64, 16 on x86_64).
type JmpBuf = [u64; 32];

/// One thread, as a `pthread_t` points at it.
pub struct Pthread {
    /// relibc's flags word, which `pthread_getattr_np` reads.
    pub(crate) flags: AtomicUsize,
    /// Always null: `std` owns the stack and does not say where it is.
    pub(crate) stack_base: *mut c_void,
    /// The size asked for at `pthread_create`, or 0 for a thread not made here.
    pub(crate) stack_size: usize,
    state: AtomicU32,
    /// `std`'s handle, until a join or a detach takes it. `None` for a thread not made here.
    handle: Mutex<Option<JoinHandle<()>>>,
    /// What the start routine returned, or what `pthread_exit` was given. Written by the thread
    /// before it ends and read by the joiner after `std`'s join, which orders the two.
    retval: UnsafeCell<*mut c_void>,
    /// Where `pthread_exit` jumps back to. Touched only by the thread itself.
    exit_env: UnsafeCell<JmpBuf>,
    /// Whether this thread was made by `pthread_create`, and so has an `exit_env` to jump to.
    created: bool,
    has_queued_cancelation: AtomicBool,
    has_enabled_cancelation: AtomicBool,
}

// SAFETY: the cells are each touched by one thread at a time, as their comments say; everything
// else is atomic or behind a lock.
unsafe impl Send for Pthread {}
// SAFETY: as above.
unsafe impl Sync for Pthread {}

impl Pthread {
    fn new(created: bool, stack_size: usize) -> Self {
        Self {
            flags: AtomicUsize::new(0),
            stack_base: ptr::null_mut(),
            stack_size,
            state: AtomicU32::new(RUNNING),
            handle: Mutex::new(None),
            retval: UnsafeCell::new(ptr::null_mut()),
            exit_env: UnsafeCell::new([0; 32]),
            created,
            has_queued_cancelation: AtomicBool::new(false),
            // POSIX: a new thread starts with cancellation enabled (and deferred).
            has_enabled_cancelation: AtomicBool::new(true),
        }
    }
}

/// A thread's return value, as relibc's module names it.
#[derive(Clone, Copy, Debug)]
pub struct Retval(pub *mut c_void);

std::thread_local! {
    /// The calling thread's record, null until first asked for on a thread not made here.
    static CURRENT: Cell<*const Pthread> = const { Cell::new(ptr::null()) };
    /// The calling thread's number (`gettid`), 0 until first asked for.
    static TID: Cell<c_int> = const { Cell::new(0) };
}

/// Threads made here that have not yet ended, which a main-thread `pthread_exit` waits for. A
/// futex word: [`thread_ended`] wakes it.
static LIVE: AtomicU32 = AtomicU32::new(0);
static NEXT_TID: AtomicI32 = AtomicI32::new(1);

/// **The calling thread's number**, 1 for the first thread to ask (normally the main thread).
pub fn thread_id() -> c_int {
    TID.with(|t| {
        if t.get() == 0 {
            t.set(NEXT_TID.fetch_add(1, Ordering::Relaxed));
        }
        t.get()
    })
}

/// The calling thread's record. A thread not made by `pthread_create` (the main thread) is given
/// one on first use, kept for the rest of the process.
pub fn current_thread() -> Option<&'static Pthread> {
    let p = CURRENT.with(|c| {
        if c.get().is_null() {
            c.set(Box::into_raw(Box::new(Pthread::new(false, 0))));
        }
        c.get()
    });
    // SAFETY: a record made here, freed only by its joiner or detacher after its thread ended,
    // and this is that thread.
    Some(unsafe { &*p })
}

unsafe extern "C" {
    /// `c_library/c/thread_start.c`: `setjmp` into `env`, then call `start(arg)`; a `longjmp` to
    /// `env` returns `*exit_value` instead.
    fn nife_pthread_trampoline(
        start: extern "C" fn(*mut c_void) -> *mut c_void,
        arg: *mut c_void,
        env: *mut u64,
        exit_value: *mut *mut c_void,
    ) -> *mut c_void;
}

/// Free a record whose thread has ended and whose handle is gone.
///
/// # Safety
/// `p` came from [`create`], and nothing will use it again.
unsafe fn free(p: *const Pthread) {
    // SAFETY: the caller's contract.
    drop(unsafe { Box::from_raw(p.cast_mut()) });
}

/// The body of a thread made by [`create`], on its own `std` thread.
fn run(p: *const Pthread, start: extern "C" fn(*mut c_void) -> *mut c_void, arg: *mut c_void) {
    CURRENT.with(|c| c.set(p));
    // SAFETY: the record outlives its thread (it is freed only once `state` says it ended), and
    // only this thread touches `exit_env` and, until it ends, `retval`.
    let this = unsafe { &*p };
    let ret = unsafe {
        nife_pthread_trampoline(start, arg, this.exit_env.get().cast(), this.retval.get())
    };
    // Thread-specific data destructors run on every way out, after `pthread_exit`'s cleanup
    // handlers (which ran before its jump) and before the thread is gone.
    // SAFETY: on this thread, which is ending.
    unsafe { header::tls::run_all_destructors() };
    // SAFETY: as above; the joiner reads it only after `std`'s join.
    unsafe { *this.retval.get() = ret };
    thread_ended();
    if this.state.swap(EXITED, Ordering::AcqRel) == DETACHED {
        // SAFETY: detached, so nobody else will free or read it; this is its last use.
        unsafe { free(p) };
    }
}

fn thread_ended() {
    LIVE.fetch_sub(1, Ordering::Release);
    // SAFETY: a static word; `wake` never dereferences it.
    let _ = unsafe { crate::platform::futex::wake(LIVE.as_ptr(), u32::MAX) };
}

/// **`pthread_create`**: a `std` thread running `start_routine(arg)`, joinable or detached as
/// `attrs` says, with the stack size it asks for.
///
/// # Safety
/// `start_routine` is a C function that may be called with `arg` on another thread.
pub(crate) unsafe fn create(
    attrs: Option<&header::RlctAttr>,
    start_routine: extern "C" fn(arg: *mut c_void) -> *mut c_void,
    arg: *mut c_void,
) -> Result<pthread_t, Errno> {
    let attrs = attrs.cloned().unwrap_or_default();
    if attrs.stack != 0 {
        // A caller-supplied stack cannot be given to `std` (BUGS).
        return Err(Errno(EINVAL));
    }
    let mut record = Pthread::new(true, attrs.stacksize);
    if c_int::from(attrs.detachstate) == header::PTHREAD_CREATE_DETACHED {
        record.state = AtomicU32::new(DETACHED);
        record.flags = AtomicUsize::new(PthreadFlags::DETACHED.bits());
    }
    let detached = record.state.load(Ordering::Relaxed) == DETACHED;
    let p: *const Pthread = Box::into_raw(Box::new(record));
    // The record holds the handle, so it must exist before the thread can end and look at it;
    // the thread does not look until `run` is done, and its `state` swap orders the rest.
    let (pp, start, a) = (p as usize, start_routine as usize, arg as usize);
    LIVE.fetch_add(1, Ordering::Relaxed);
    let spawned = std::thread::Builder::new()
        .stack_size(attrs.stacksize)
        .spawn(move || {
            // SAFETY: the same function pointer, carried as an integer to be `Send`.
            let start: extern "C" fn(*mut c_void) -> *mut c_void =
                unsafe { core::mem::transmute(start) };
            run(pp as *const Pthread, start, a as *mut c_void)
        });
    match spawned {
        Ok(handle) => {
            if detached {
                // Dropping `std`'s handle is its detach; the thread frees the record.
                drop(handle);
            } else {
                // SAFETY: a joinable record lives until joined or detached, both after this.
                let this = unsafe { &*p };
                *this.handle.lock().unwrap_or_else(|e| e.into_inner()) = Some(handle);
            }
            Ok(p.cast_mut().cast())
        }
        Err(_) => {
            LIVE.fetch_sub(1, Ordering::Relaxed);
            // SAFETY: no thread was made, so nothing else has it.
            unsafe { free(p) };
            Err(Errno(EAGAIN))
        }
    }
}

/// **`pthread_join`**: wait for `thread` to end, free it, and answer what it returned.
///
/// # Safety
/// `thread` came from [`create`] and has been neither joined nor detached.
pub unsafe fn join(thread: &Pthread) -> Result<Retval, Errno> {
    if current_thread().is_some_and(|me| ptr::eq(thread, me)) {
        return Err(Errno(EDEADLK));
    }
    let handle = thread
        .handle
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
        .ok_or(Errno(EINVAL))?;
    // `std`'s join returns once the thread has left for good, which orders its `retval` write
    // before the read below. The thread cannot panic: `panic = "abort"`.
    let _ = handle.join();
    // SAFETY: the thread has ended.
    let ret = unsafe { *thread.retval.get() };
    // SAFETY: joined, so the thread is gone and no detach can follow (POSIX: a joined thread's
    // `pthread_t` is no longer valid).
    unsafe { free(thread) };
    Ok(Retval(ret))
}

/// **`pthread_detach`**: `thread` frees itself when it ends, or is freed now if it has.
///
/// # Safety
/// `thread` came from [`create`] or [`current_thread`] and has been neither joined nor detached;
/// it is not used again after this returns.
pub unsafe fn detach(thread: &Pthread) -> Result<(), Errno> {
    if !thread.created {
        // The main thread: nothing ever joins it, and its record is kept anyway.
        return Ok(());
    }
    let handle = thread
        .handle
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
        .ok_or(Errno(EINVAL))?;
    thread
        .flags
        .fetch_or(PthreadFlags::DETACHED.bits(), Ordering::AcqRel);
    match thread
        .state
        .compare_exchange(RUNNING, DETACHED, Ordering::AcqRel, Ordering::Acquire)
    {
        // Still running: it frees its own record when it ends.
        Ok(_) => drop(handle),
        Err(_) => {
            // Already ended: reap it now, as a join would.
            let _ = handle.join();
            // SAFETY: ended and its handle taken, so nothing else has it.
            unsafe { free(thread) };
        }
    }
    Ok(())
}

/// **`pthread_testcancel`**: end the calling thread if a cancel is pending and enabled.
///
/// # Safety
/// As [`exit_current_thread`]: the caller's frames hold nothing that must be dropped.
pub unsafe fn testcancel() {
    let this = current_thread().expect("current thread not present");
    if this.has_queued_cancelation.load(Ordering::Acquire)
        && this.has_enabled_cancelation.load(Ordering::Acquire)
    {
        // SAFETY: called on the thread being cancelled.
        unsafe { exit_current_thread(Retval(header::PTHREAD_CANCELED)) };
    }
}

/// **`pthread_exit`**: run the cleanup handlers, then end the calling thread with `retval`.
///
/// # Safety
/// Every frame between here and the thread's start is C or holds nothing with a destructor:
/// `longjmp` skips them all.
pub unsafe fn exit_current_thread(retval: Retval) -> ! {
    // `pthread_cleanup_push` handlers, newest first.
    // SAFETY: on the exiting thread.
    unsafe { header::run_destructor_stack() };
    let this = current_thread().expect("current thread not present");
    if this.created {
        // SAFETY: only this thread touches its own `retval` before it ends, and `exit_env` was
        // filled by the `setjmp` in `thread_start.c` below every frame `longjmp` discards. Nothing
        // in this frame has a destructor left to run.
        unsafe {
            *this.retval.get() = retval.0;
            crate::header::setjmp::longjmp(this.exit_env.get().cast(), 1);
            // `longjmp` does not return.
            core::hint::unreachable_unchecked()
        }
    }
    // The main thread: its thread-specific data goes now, then the process ends once every
    // other thread made here has.
    // SAFETY: on the exiting thread.
    unsafe { header::tls::run_all_destructors() };
    loop {
        let live = LIVE.load(Ordering::Acquire);
        if live == 0 {
            break;
        }
        // SAFETY: a static, aligned word changed only atomically.
        let _ = unsafe { crate::platform::futex::wait(LIVE.as_ptr(), live, None) };
    }
    // SAFETY: the C library's `exit`, which runs `atexit` handlers and flushes the C streams.
    unsafe { crate::header::stdlib::exit(0) }
}

/// **`pthread_cancel`**: mark `thread` cancelled, for its next `pthread_testcancel`.
///
/// # Safety
/// `thread` names a live thread of this process (relibc's signature; nothing here needs more).
pub unsafe fn cancel(thread: &Pthread) -> Result<(), Errno> {
    // Deferred only: the thread acts on it at its next `pthread_testcancel` (BUGS).
    thread.has_queued_cancelation.store(true, Ordering::Release);
    Ok(())
}

pub fn set_sched_param(
    _thread: &Pthread,
    _policy: c_int,
    _param: &sched_param,
) -> Result<(), Errno> {
    // Accepted and ignored: one scheduling class (BUGS).
    Ok(())
}

pub fn set_sched_priority(_thread: &Pthread, _prio: c_int) -> Result<(), Errno> {
    Ok(())
}

pub fn set_cancel_state(state: c_int) -> Result<c_int, Errno> {
    let this = current_thread().expect("current thread not present");
    let was_enabled = match state {
        header::PTHREAD_CANCEL_ENABLE => {
            let old = this.has_enabled_cancelation.swap(true, Ordering::Release);
            if this.has_queued_cancelation.load(Ordering::Acquire) {
                // SAFETY: on the thread being cancelled.
                unsafe { exit_current_thread(Retval(header::PTHREAD_CANCELED)) };
            }
            old
        }
        header::PTHREAD_CANCEL_DISABLE => {
            this.has_enabled_cancelation.swap(false, Ordering::Release)
        }
        _ => return Err(Errno(EINVAL)),
    };
    Ok(if was_enabled {
        header::PTHREAD_CANCEL_ENABLE
    } else {
        header::PTHREAD_CANCEL_DISABLE
    })
}

pub fn set_cancel_type(ty: c_int) -> Result<c_int, Errno> {
    match ty {
        // Asynchronous cancellation is accepted and behaves as deferred (BUGS).
        header::PTHREAD_CANCEL_DEFERRED | header::PTHREAD_CANCEL_ASYNCHRONOUS => {
            Ok(header::PTHREAD_CANCEL_DEFERRED)
        }
        _ => Err(Errno(EINVAL)),
    }
}

pub fn get_cpu_clkid(_thread: &Pthread) -> Result<clockid_t, Errno> {
    // nife accounts no CPU time to a thread.
    Err(Errno(ENOENT))
}

pub fn get_sched_param(_thread: &Pthread) -> Result<(clockid_t, sched_param), Errno> {
    Err(Errno(ENOSYS))
}

/// Whether a lock is private to a process or shared between processes, as relibc's module names
/// it. Taken unchanged from relibc's `pthread` module (MIT).
#[derive(Clone, Copy, Default, Debug)]
pub enum Pshared {
    #[default]
    Private,

    Shared,
}

impl Pshared {
    pub const fn from_raw(raw: c_int) -> Option<Self> {
        Some(match raw {
            header::PTHREAD_PROCESS_PRIVATE => Self::Private,
            header::PTHREAD_PROCESS_SHARED => Self::Shared,

            _ => return None,
        })
    }

    pub const fn raw(self) -> c_int {
        match self {
            Self::Private => header::PTHREAD_PROCESS_PRIVATE,
            Self::Shared => header::PTHREAD_PROCESS_SHARED,
        }
    }
}
