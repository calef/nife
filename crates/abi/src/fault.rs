/// **The spawn-slot convention.** A supervised child is spawned with its supervision endpoint
/// in this reserved capability table slot (via [`crate::thread_control_block::CAP_INSERT`] with an explicit target slot, or a
/// [`Spawn`](../user/struct.Spawn.html) grant). At `START` the kernel reads this slot: if it
/// holds a `Rendezvous` capability the thread is supervised, and the kernel records the endpoint
/// as the thread's fault target and clears the slot (so the child cannot forge fault messages
/// on it, keeping §26's "the kernel is the only sender" property). An empty slot means the
/// thread is unsupervised and gets today's behaviour: it dies and is reaped immediately.
///
/// It is the **last** capability table slot, deliberately out of the way of the low slots a child's
/// ordinary grants fill from zero upward, so an unsupervised child never accidentally lands a
/// working endpoint here and gets mistaken for a supervised one.
pub const FAULT_EP_SLOT: u64 = super::CAPABILITY_TABLE_SLOTS - 1;

// "The last slot" is the whole claim above; a wrong operator in the line before would still
// compile, so this makes it one that does not (a mutation-survivor triage).
const _: () = assert!(FAULT_EP_SLOT + 1 == super::CAPABILITY_TABLE_SLOTS);

/// **The message-format convention.** A fault/exit notification is five words, delivered to the
/// supervision endpoint's holder through a plain `RECEIVE`:
///
/// ```text
///   w0  event    FAULT or EXIT
///   w1  tid      the dead thread's id (kernel-stamped, trustworthy: the kernel is the
///                only sender on this path, so the supervisor need not badge it)
///   w2  pc       the faulting instruction (0 for a clean exit)
///   w3  addr     the faulting address (0 for a clean exit, or a fault with no address)
///   w4  reserved 0 today; a fault-reply / resume protocol arrives here additively (§26.4)
/// ```
///
/// `RECEIVE` returns w0 in the syscall's result register and w1..w4 in the next four argument
/// registers. Ordinary three-word IPC leaves w3 and w4 zero, so a supervisor is the only
/// receiver that reads them.
///
/// **A sixth word, the label, rides beside the five** (milestone 105 (the two
/// forks), DECISIONS §148 (resolves by asking the kernel) as amended 2026-10-04, ruling R3):
///
/// ```text
///   r5  label    the badge the builder put on the child's supervision capability with
///                rendezvous::BADGE before inserting it in FAULT_EP_SLOT; 0 if it was unbadged
/// ```
///
/// `r5` is argument register 5: `x5` on aarch64, `a5` on riscv64, `r9` on `x86_64`. The kernel
/// keeps the badge when `START` consumes the slot and writes it there on a death delivered to a
/// plain `RECEIVE`, and on nothing else, so ordinary IPC stores no extra word. A receiver that
/// wants `0` to mean "not a kernel-stamped death" zeroes the register before the `RECEIVE`, as
/// `user_mode_runtime::receive_fault` does. The child never holds a capability carrying its
/// label, and the badge on a capability a sender uses arrives in `w3`, never in `r5`, so a
/// label can be neither learned nor forged by the child it names.
pub const EVENT_FAULT: u64 = 1;
/// `w0`: the thread called `exit` (or was reaped), rather than faulting.
pub const EVENT_EXIT: u64 = 2;
