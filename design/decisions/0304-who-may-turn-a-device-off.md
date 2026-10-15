---
status: PROPOSED
raised: 2026-10-15
---

# 304. Who may turn a device off, and what the unit of reclamation is

Raised 2026-10-15 by the lane for milestone 652 (twelve `DECISION` gates cite only sections that
are already ruled). Milestone 376 (nothing turns a device back off) was filed by
[milestone 220 (this kernel drives no clock or reset controller)](../roadmap/0220-jh7110-clock-and-reset.md)'s
lane. That lane wrote the first code in this tree that turns a device on, and deliberately did not
write its inverse. A driver process that exits, faults or is revoked leaves its device clocked and running
forever, and nothing in the system can reclaim that. *(Section number provisional until the merge
queue lands it.)*

## What is being decided

Three questions, in the order the block asked them:

1. Who may gate a clock or assert a reset: a driver holding a capability, or nobody but the kernel.
2. What the unit of reclamation is: per device, with a refcount for shared lines, or per job, off
   [§40 (a supervisor's death is its subtree's death)](0040-no-reaper-of-last-resort.md)'s subtree death.
3. Whether any of it is worth building before somebody measures what an ungated STG domain costs
   radon.

## What this tree already does in the analogous case

Milestone 220 kept the whole clock-and-reset controller in the kernel, on the recorded argument
that granting it would widen a driver's authority rather than confine it. The STG window covers
USB, both PCIe root ports and the DMA engine, not one device. The crate
(`crates/jh7110_clock_and_reset`) exposes `discover`, the offset and mask arithmetic and the two
read-back predicates, and offers no gate and no assert. Its own doc for the TRNG reset line says
why the asymmetry is the silicon's: the line is *shared* (`devm_reset_control_get_shared`
upstream), the same line resets the PL080 DMA engine, so "deasserting it is safe; asserting it
would reset a neighbour, which is why nothing in this crate offers an assert". Linux's clock
framework buys its way out with refcounting this tree does not have.

[§86 (whether an NVMe driver can leave the kernel)](0086-el0-nvme-driver.md) is the tree's own precedent for the shape of the answer, and its words fit: "the irreversible
part of this decision is not in 2a. It costs zero new syscall surface", and the choice between the
cheap answer and the capability was "settled by measurement". [§92 (a caretaker is supervised by the client it serves)](0092-caretaker-lifetime.md) is the precedent for question 2:
the caretaker got no new lifetime concept because §40's subtree death already collects it. The
kernel knows when a spawned service dies; that is the one reclamation hook in the system that
already fires.

Prior art, read rather than recalled: the shared-reset fact and the refcounting are recorded in
`notes/jh7110-clock-and-reset.md` from Linux's source, and the note's BUGS section carries the
limitation beside the feature: "nothing here can turn a clock off or assert a reset, deliberately.
... It also means there is no teardown: a service that dies leaves its device clocked."

## The options

| | shape | what it costs |
|---|---|---|
| **A** | A driver may ask: a capability variant on the syscall surface for gating and asserting. | New surface in the category [§10 (process model)](0010-capability-microkernel.md) and [§16 (object revocation)](0016-object-revocation.md) exist to keep narrow, granted against a window that reaches devices the holder has no business resetting. |
| **B** | The kernel reclaims at job death: a spawned job's enables are recorded where they are made, and the kernel undoes exactly that job's enables, refcounted per shared line, when its subtree dies. | Bookkeeping in the kernel's controller path and a refcount, and no new surface at all; the driver never learns that power management happened. |
| **C** | Measure before building: radon runs with the STG domain ungated and gated, and the number decides whether anything downstream of this question is worth a milestone. | A bench session and a person at the board, and the honest possibility that the answer is "not yet worth it", which is a real answer. |

Recommendation: C, then B if the number justifies. A waits for a driver that ever needs to
power-manage a device the kernel's controller did not enable. The reasoning is the block's own,
and it is three facts rather than a mood. The teardown a driver asks for is a different question
from a controller a driver holds, and the first has no consumer in this tree. The per-job unit is
the more elegant of the two reclaim shapes, and it needs the refcount anyway, so B contains what A
would have needed of mechanism. And a milestone justified by tidiness rather than a number is the
implementation-convenience tenet running backwards. §86 is the precedent for spending the
measurement before spending the surface.

The effort test, stated because the skill asks: B is more work than A's minimal ask and would
still be the choice. A spends irrevocable surface on a capability whose consumer does not exist.
B spends reversible bookkeeping on a path the kernel already walks at every death.

## Whether the premise is true

Checked 2026-10-15. The crate still offers no gate and no assert, the shared-reset doc still says
why, and the note's BUGS still carries the no-teardown limitation. Nothing in `design/decisions/`
takes up who may turn a device off. The three questions are open exactly as the block left them.

## How reversible it is

C is a bench run. B is kernel bookkeeping on a path that already exists, removable without a
trace. A is the syscall surface, which is the one irreversible thing in this question, and it is
the part with no consumer. So the order of trying is fixed by reversibility, and the surface stays
out of reach until a measured number and a real driver both ask for it.

## What is blocked until this is answered

Milestone 376, and nothing else in the tree: no other block waits on device power management
today, which is itself evidence for question 3. The BUGS entry beside the driver stays either way,
since it records a fact about the hardware the crate serves.
