/// `invoke(cap, CONFIGURE, entry, user_sp, address_space_slot)` -> 0. Bind the address space
/// named by the capability in `address_space_slot`, and set where EL0 execution begins and on
/// what user stack. Needs `WRITE` on the TCB cap and `WRITE` on the address-space cap. Only an
/// unstarted (embryo) TCB.
///
/// **The capability passed is consumed; the space's name is not** (§249 (a running address
/// space stays nameable), option A, 2026-10-05). A copy made before `CONFIGURE` goes on naming
/// the space while the thread runs, which is how a builder that chooses to keep one (or a
/// process granted its own, as the progenitor is at boot) can [`UNMAP`](super::address_space::UNMAP) a page
/// of a running space. The space dies when its thread is reaped or the region its root came
/// from is destroyed, and every capability still naming it then fails; deleting a capability
/// never frees it.
///
/// **A space that already has a holder is refused with `WrongObject`**: a thread that is a
/// process of its own, or a process. Threads share a space by joining its process (below), so
/// §249's amendment (b) no longer stands between a space and a second thread (#1892).
///
/// **The sixth argument register is the thread's first thread pointer** (milestone 812
/// (`std::thread::spawn` runs real threads in one address space), §269 (how threads share a
/// process) fork 4; Linux's `CLONE_SETTLS`): `TPIDR_EL0`, `tp` or the `FS` base, which the
/// kernel then keeps per thread. `CONFIGURE` is the one method that reads that register. Zero
/// means none, which is what every caller built before 812 sends, since
/// `user_mode_runtime::invoke6` zeroes the word. A value that is neither zero nor a user
/// address is refused with `BadPointer` and nothing is bound. *(Encoding ruled by calef on pull
/// request #1892, question 2; number unchanged.)*
///
/// **The result is the thread's current-CPU page address** (milestone 812, §269 fork 6, the same
/// ruling), never negative: the page the kernel publishes this thread's core and its allowance
/// on. A thread configured with a space reads `current_cpu_contract::PAGE_VA`; one that joins a
/// process reads its own slot below it (`current_cpu_contract::thread_page_va`). Zero if the space
/// could not be given a page.
///
/// **Given a process capability with [`rights::BIND`](super::rights::BIND) instead of an address
/// space**, the thread joins that process and the capability is not consumed (§269 fork 1, ruling
/// (b) on #1892): see [`super::process`]. `OutOfMemory` when the process already has
/// `current_cpu_contract::THREAD_PAGE_SLOTS` threads.
pub const CONFIGURE: u64 = 0;

/// `invoke(cap, CAP_INSERT, cap_slot, rights, target)` -> `child_slot`. Copy the capability in
/// the caller's `cap_slot`, narrowed to `rights`, into the child's capability table, returning the slot
/// it landed in. The child's whole initial authority is built this way, one grant at a time,
/// before it runs. Needs `WRITE` on the TCB cap and `GRANT` on the inserted capability.
///
/// `target` chooses where the capability lands: **0 places it in the first free slot** (the
/// original behaviour, so every existing caller is unchanged), and `n` places it in slot
/// `n - 1`. The explicit target exists for the supervision endpoint, which a supervisor puts
/// in the reserved [`fault::FAULT_EP_SLOT`](super::fault::FAULT_EP_SLOT) rather than wherever
/// first-free happened to fall. `OutOfMemory` if the chosen slot is occupied or out of range.
pub const CAP_INSERT: u64 = 1;

/// `invoke(cap, START, x0, x1, x2)` -> 0. Make the thread runnable: it gets a kernel stack and
/// an entry context and joins the run queue. **The three arguments are the child's first
/// `x0`, `x1` and `x2`** (`a0` to `a2` on RISC-V, `rdi`, `rsi` and `rdx` on `x86_64`), passed
/// through unconditionally, which is how a builder hands a worker its input
/// (milestone 19 (run a real workload), phases d and e). A caller with nothing to pass sends
/// zeros. **Refuses a half-built thread** (no bound address space, or no entry): a TCB must be
/// whole before it runs. Needs `WRITE`.
pub const START: u64 = 2;

/// `invoke(cap, SET_THREAD_POINTER, value, _, _)` -> 0 (milestone 812, §269 (how threads share
/// a process) fork 4; seL4's `SetTLSBase`). **Change the thread pointer of the thread this TCB
/// names**: `TPIDR_EL0` on aarch64, `tp` on riscv64, the `FS` base on `x86_64`. Needs `WRITE`.
///
/// - The target is an embryo (it takes the value when it first runs) or the caller itself (it
///   takes it before this call returns). Any other started thread is refused with
///   `WrongObject`.
/// - `value` is zero or a user address; anything else is `BadPointer` and changes nothing.
/// - The kernel keeps the value per thread and installs it at every switch, so a sibling never
///   sees it. On aarch64 and riscv64 the hardware also lets a program write its own register,
///   which is not part of this contract (`notes/thread-pointer.md`).
///
/// *(Name and number provisional, milestone 812's lane, 2026-10-10 UTC.)*
pub const SET_THREAD_POINTER: u64 = 3;

/// `invoke(cap, SUSPEND, _, _, _)` -> 0 (DECISIONS §300 (the paused state, a TCB verb, silent);
/// seL4's `TCB_Suspend`). **Stop the thread this TCB names until a [`RESUME`]**, keeping its
/// registers. Needs `WRITE`: the TCB capability is the whole authority.
///
/// - **Silent.** No message, no event code, nothing on the supervision endpoint that §26 (the
///   fault endpoint) reports deaths on. The caller is the witness.
/// - **Stopped when it returns.** A thread running on another core is taken off it before the call
///   answers, so a caller that reads the thread afterwards sees it standing still.
/// - **A thread blocked in IPC stays in that IPC.** Its rendezvous may still complete, and it then
///   stops before running another user instruction; a `RESUME` lets it return what it was given.
/// - A thread may suspend itself; the call returns 0 after somebody resumes it.
/// - Suspending a suspended thread is 0 and changes nothing. An embryo is `WrongObject` (`START`
///   it); a thread that has exited or died is `Gone`.
///
/// Name: ratified 2026-10-18 (calef: "Yes"), as §300 rules.
pub const SUSPEND: u64 = 4;

/// `invoke(cap, RESUME, _, _, _)` -> 0 (DECISIONS §300; seL4's `TCB_Resume`). **Let a suspended
/// thread run on from where it stopped**: the saved PC, every register as it was. Not a re-run.
/// Needs `WRITE`.
///
/// Resuming a thread that is not suspended is 0 and changes nothing. An embryo is `WrongObject`; a thread that
/// has exited or died is `Gone`.
///
/// Name: ratified 2026-10-18 (calef: "Yes"), as §300 rules.
pub const RESUME: u64 = 5;
