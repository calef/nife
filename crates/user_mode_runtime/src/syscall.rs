//! **The trap**: the one six-register round trip per architecture every syscall shares, and the raw
//! and capability-invoking wrappers over it.
//!
//! Cut out of `lib.rs` unchanged on 2026-10-12 (UTC) by lane maintainer/files-over-1500, for §282
//! (when a file is split, the parent becomes an index); the crate root re-exports every public
//! item, so each keeps its path.
//!
//! Name: provisional, lane maintainer/files-over-1500 (#2017), 2026-10-12 (UTC), for the module and
//! the file. Named for the calls it holds, one module per seam of the old crate root; the pull
//! request records no other name considered.

/// **The one trap in this crate**, one per architecture, used by every syscall since 2026-10-05:
/// the number in `x8`/`a7`/`rax` and six words in both directions. The kernel writes its result
/// into the first register on the way out of *every* syscall, and some methods write up to the
/// sixth. Until 2026-10-05 [`yield_now`] and [`cap_delete`] had their own `asm!` blocks declaring
/// no output, a promise that `x0` survives the trap. It does not: a yield comes back with `x0 = 0`.
/// The job mix's spawn job had its retry loop compiled with `DESTROY`'s arguments loaded before
/// the yield, so the retry sent `invoke(0, 4)`, a `CALL` on the report endpoint, answered `Gone`
/// (notes/job-mix/spawn-destroy-gone.md). `helpers/syscall_asm.py` now gates every userspace trap.
/// What follows is this primitive's `SYS_INVOKE` history.
///
/// The raw six-register round trip through `SYS_INVOKE` (milestone 139 (drive the unsafe count down) round
/// 2; the sixth register is milestone 105 (the two forks)'s). `cap`, `method`
/// and two more arguments go in `x0..x3`/`a0..a3` (the fifth, `x4`/`a4`, is spare and always zero on
/// input); the kernel's reply comes back in the same five registers, `x0..x4`/`a0..a4`. This is now
/// the one place the actual trap instruction and register file appear for a `SYS_INVOKE` call:
/// [`invoke`] and every multi-word method below ([`receive`], [`receive_cap`], [`receive_fault`], [`call`],
/// [`survey`], [`list`]) used to each hand-roll their own `asm!` block asserting the identical
/// invariant ("`svc`/`ecall` traps to the kernel, which validates before acting") at a register
/// layout that differed only in which of the five words the caller happened to read back. Six
/// functions, two architectures, twelve hand-written copies of one assertion: the §94 shape this
/// milestone names as the reduction worth making. Now there are two, one per architecture, and
/// every caller above is a safe wrapper that just picks which return words it wants.
///
/// One behavioural note for a reader diffing this against the asm the individual functions used to
/// carry: a few of them ([`receive`], [`receive_cap`], [`receive_fault`]) left `x2`/`a2` with no `in`
/// operand at all, so the kernel received whatever value happened to already be in that register
/// (harmless, since `RECEIVE`/`RECEIVE_CAP` read no input words). Routing them through this shared
/// primitive means they now pass an explicit `0` there instead, which is a strict tightening, not a
/// behaviour change: the kernel still ignores it.
///
/// **The sixth register is an output, and goes in as zero** (milestone 105, DECISIONS §148 (resolves by
/// asking the kernel) as amended 2026-10-04). The kernel writes argument register 5 (`x5`, `a5`, `r9`) with a dead
/// child's label when it delivers a death message to a plain `RECEIVE`, and leaves it alone on
/// every other path. Declared as an input only, LLVM would assume the register survived the trap
/// and keep a live value in it that a `RECEIVE` on a supervision endpoint then silently replaced.
/// Zero in makes the result defined everywhere: [`receive_fault`] reads `0` for anything the
/// kernel did not stamp. One `mov` per call, in userspace; the kernel paths `script/bench` gates
/// run nothing new.
///
/// # Safety
/// `svc`/`ecall` traps to the kernel. The kernel validates the capability and the method before
/// acting; that is its whole job. The caller is trusting the kernel, not the other way around.
///
/// Name: provisional, `lane/hvf-spawn-destroy-gone`, 2026-10-05 (UTC): the trap every syscall
/// shares. It began as milestone 105 (the two forks)'s `invoke6`, which is now a wrapper over it.
// Since 2026-10-12 the callers this names as "below" and "above" live in `ipc.rs`,
// `objects.rs` and `process.rs`, and the crate root re-exports each at its old path.
#[cfg(target_arch = "aarch64")]
pub(crate) unsafe fn trap6(nr: u64, a: [u64; 6]) -> (u64, u64, u64, u64, u64, u64) {
    let (mut w0, mut w1, mut w2, mut w3, mut w4, mut w5): (u64, u64, u64, u64, u64, u64);
    // SAFETY: see the function doc; `x8` selects the syscall (DECISIONS §10 (process model:
    // capability-based, microkernel)), `x0..x5` carry the six-word ABI in both directions. `asm!`
    // is unsafe because the compiler cannot check that, not because a caller can get it wrong.
    unsafe {
        core::arch::asm!(
            "svc #0",
            in("x8") nr,
            inlateout("x0") a[0] => w0,
            inlateout("x1") a[1] => w1,
            inlateout("x2") a[2] => w2,
            inlateout("x3") a[3] => w3,
            inlateout("x4") a[4] => w4,
            inlateout("x5") a[5] => w5,
            options(nostack),
        );
    }
    (w0, w1, w2, w3, w4, w5)
}

/// The raw five-register round trip (RISC-V). See the aarch64 twin's doc for the contract this
/// collapses; only the trap instruction and register file differ: `ecall`, number in `a7`, the five
/// words in `a0..a4`.
///
/// # Safety
/// `ecall` traps to the kernel, which validates the capability and method before acting. Same
/// contract as the aarch64 twin: the caller trusts the kernel, not the other way around.
///
/// Name: provisional, `lane/hvf-spawn-destroy-gone`, 2026-10-05 (UTC): the trap every syscall
/// shares. It began as milestone 105 (the two forks)'s `invoke6`, which is now a wrapper over it.
#[cfg(target_arch = "riscv64")]
pub(crate) unsafe fn trap6(nr: u64, a: [u64; 6]) -> (u64, u64, u64, u64, u64, u64) {
    let (mut w0, mut w1, mut w2, mut w3, mut w4, mut w5): (u64, u64, u64, u64, u64, u64);
    // SAFETY: see the function doc; `a7` selects the syscall (DECISIONS §10), `a0..a5` carry the
    // six-word ABI in both directions.
    unsafe {
        core::arch::asm!(
            "ecall",
            in("a7") nr,
            inlateout("a0") a[0] => w0,
            inlateout("a1") a[1] => w1,
            inlateout("a2") a[2] => w2,
            inlateout("a3") a[3] => w3,
            inlateout("a4") a[4] => w4,
            inlateout("a5") a[5] => w5,
            options(nostack),
        );
    }
    (w0, w1, w2, w3, w4, w5)
}

/// The raw five-register round trip (`x86_64`, milestone 161). See the aarch64 twin's doc for the
/// contract this collapses; only the trap instruction and register file differ. `syscall`, the
/// number in `rax`, the five words in `rdi`, `rsi`, `rdx`, `r10`, `r8` (DECISIONS §124).
///
/// **Two operands here have no counterpart on the other two architectures**, and both are the
/// instruction rather than a choice. `syscall` writes the return address into `rcx` and the
/// caller's `RFLAGS` into `r11`, unconditionally, so both are declared clobbered; a version of
/// this without them compiles and then corrupts whichever local the register allocator had put
/// there. And `r10` carries the fourth word instead of the C ABI's `rcx` for exactly the same
/// reason, which is why §124 records that substitution as forced rather than preferred.
///
/// `options(nostack)` still holds: `syscall` does not push, which is the whole reason the kernel's
/// entry path has to park `rsp` by hand (see `arch/x86_64/trap.s`).
///
/// # Safety
/// `syscall` traps to the kernel, which validates the capability and method before acting. Same
/// contract as the aarch64 twin: the caller trusts the kernel, not the other way around.
///
/// Name: provisional, `lane/hvf-spawn-destroy-gone`, 2026-10-05 (UTC): the trap every syscall
/// shares. It began as milestone 105 (the two forks)'s `invoke6`, which is now a wrapper over it.
#[cfg(target_arch = "x86_64")]
pub(crate) unsafe fn trap6(nr: u64, a: [u64; 6]) -> (u64, u64, u64, u64, u64, u64) {
    let (mut w0, mut w1, mut w2, mut w3, mut w4, mut w5): (u64, u64, u64, u64, u64, u64);
    // SAFETY: see the function doc; `rax` selects the syscall (DECISIONS §10, §124 (the `x86_64`
    // syscall ABI)), and the six argument registers carry the six-word ABI in both directions.
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") nr,
            inlateout("rdi") a[0] => w0,
            inlateout("rsi") a[1] => w1,
            inlateout("rdx") a[2] => w2,
            inlateout("r10") a[3] => w3,
            inlateout("r8") a[4] => w4,
            inlateout("r9") a[5] => w5,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack),
        );
    }
    (w0, w1, w2, w3, w4, w5)
}

/// **One syscall with every register the caller's to choose**: [`trap6`], public, for the one
/// program that must be able to say anything to the kernel (milestone 779 (fuzz the surface a
/// confined process can reach), provisional). `fixtures/src/confined_syscall_fuzzer.rs` draws the
/// number and all six words at random, unknown numbers included, which no typed wrapper here can
/// express. Every other caller wants a typed wrapper, and this is not one.
///
/// # Safety
/// [`trap6`]'s: the kernel validates the capability and the method before acting. A caller that
/// passes `SYS_EXIT` does not come back.
///
/// Name: provisional, milestone 779's lane, 2026-10-06 (UTC).
pub unsafe fn raw_syscall(number: u64, words: [u64; 6]) -> [u64; 6] {
    // SAFETY: forwarded from this function's own contract.
    let (w0, w1, w2, w3, w4, w5) = unsafe { trap6(number, words) };
    [w0, w1, w2, w3, w4, w5]
}

/// The six-register round trip through `SYS_INVOKE`: [`trap6`] with the number fixed and the sixth
/// word in as zero, so [`receive_fault`](crate::receive_fault) reads `0` for anything the kernel
/// did not stamp.
///
/// # Safety
/// [`trap6`]'s: the kernel validates the capability and the method before acting.
///
/// Name: provisional, milestone 105 (the two forks)'s lane, 2026-10-05 (UTC).
#[inline(always)]
pub(crate) unsafe fn invoke6(
    cap: u64,
    method: u64,
    a0: u64,
    a1: u64,
    a2: u64,
) -> (u64, u64, u64, u64, u64, u64) {
    // SAFETY: forwarded from this function's own contract.
    unsafe { trap6(abi::SYS_INVOKE, [cap, method, a0, a1, a2, 0]) }
}

/// [`invoke6`] read to the five words every method but a death delivery uses.
///
/// # Safety
/// Exactly [`invoke6`]'s contract.
#[inline(always)]
pub(crate) unsafe fn invoke5(
    cap: u64,
    method: u64,
    a0: u64,
    a1: u64,
    a2: u64,
) -> (u64, u64, u64, u64, u64) {
    // SAFETY: forwarded from this function's own contract.
    let (w0, w1, w2, w3, w4, _) = unsafe { invoke6(cap, method, a0, a1, a2) };
    (w0, w1, w2, w3, w4)
}

/// Invoke a capability: the one syscall a userspace program makes. `cap` names a capability in the
/// process's capability table, `method` selects the operation, and `a0..a2` are its arguments; the return is
/// the kernel's `i64` result. Everything else in this crate is built on this.
///
/// # Safety
/// `svc`/`ecall` traps to the kernel. The kernel validates the capability and the method before
/// acting; that is its whole job. The caller is trusting the kernel, not the other way around.
pub unsafe fn invoke(cap: u64, method: u64, a0: u64, a1: u64, a2: u64) -> i64 {
    // SAFETY: forwarded from this function's own contract, which is `invoke5`'s contract exactly.
    unsafe { invoke5(cap, method, a0, a1, a2).0 as i64 }
}
