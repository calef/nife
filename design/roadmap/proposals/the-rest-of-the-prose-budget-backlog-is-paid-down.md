---
status: PROPOSED
raised: 2026-10-12
milestone_dependencies: none
decision_dependencies: 212
machine_requirements: none
specific_machine: none
needs_person: no
---
# The rest of the prose budget's backlog is paid down

Raised on 2026-10-12 (UTC) by the lane `maintainer/prose-budget-paydown`, which calef asked for that
day. That lane and its sibling `maintainer/prose-budget-decisions` (#1992 and #1994) split the
fifteen roadmap blocks and six decisions with the most words over the 3,000-word cap of
[§212 (a prose budget)](../../decisions/0212-a-prose-budget-for-every-document.md). This file is the
work they found and did not do.

Reuse: no code is written. The tools are the tree's own: `helpers/prose_ratchet.py` measures and
banks, and the per-document method is the one #1992 and #1994 followed.

## What is left

Measured with `helpers/prose_ratchet.py` on 2026-10-12, with both pull requests applied: 38 documents
outside the two marked exceptions, holding 27,634 words over the cap. The exceptions are calef's for
[`0047-navigation-and-naming.md`](../0047-navigation-and-naming.md) and
[`0139-drive-down-unsafe.md`](../0139-drive-down-unsafe.md). They are not in this list.

### Roadmap blocks, 22,861 words over

| block | words over the cap |
|---|---|
| [`0533-the-roadmap-after-the-split.md`](../0533-the-roadmap-after-the-split.md) | 1,567 |
| [`0064-std-for-real-crates.md`](../0064-std-for-real-crates.md) | 1,551 |
| [`0187-x86-64-arch-audit.md`](../0187-x86-64-arch-audit.md) | 1,460 |
| [`0168-multitasking-benchmark.md`](../0168-multitasking-benchmark.md) | 1,364 |
| [`0062-time-sensitive-tests.md`](../0062-time-sensitive-tests.md) | 1,329 |
| [`0057-partitioning-and-xattrs.md`](../0057-partitioning-and-xattrs.md) | 1,309 |
| [`0074-cycle-counters.md`](../0074-cycle-counters.md) | 1,141 |
| [`0584-does-this-layer-earn-its-keep.md`](../0584-does-this-layer-earn-its-keep.md) | 1,129 |
| [`0158-kernel-object-rename-build.md`](../0158-kernel-object-rename-build.md) | 1,100 |
| [`0040-documentation-service.md`](../0040-documentation-service.md) | 1,055 |
| [`0243-a-machine-with-no-serial-port-cannot-say-anything.md`](../0243-a-machine-with-no-serial-port-cannot-say-anything.md) | 913 |
| [`0517-survivor-growth-on-the-lines-a-diff-touched.md`](../0517-survivor-growth-on-the-lines-a-diff-touched.md) | 893 |
| [`0124-a-thread-is-born-where-it-lives.md`](../0124-a-thread-is-born-where-it-lives.md) | 881 |
| [`0198-package-manager.md`](../0198-package-manager.md) | 852 |
| [`0326-turn-a-mutation-score-upward.md`](../0326-turn-a-mutation-score-upward.md) | 743 |
| [`0317-interrupt-remapping-flags.md`](../0317-interrupt-remapping-flags.md) | 668 |
| [`0266-init-is-an-action-and-the-thing-is-a-process.md`](../0266-init-is-an-action-and-the-thing-is-a-process.md) | 622 |
| [`0587-ci-arch-split.md`](../0587-ci-arch-split.md) | 553 |
| [`0165-x86-64-pci-acpi-mcfg.md`](../0165-x86-64-pci-acpi-mcfg.md) | 552 |
| [`0580-nobody-reads-branches.md`](../0580-nobody-reads-branches.md) | 547 |
| [`0192-keyboard-on-real-silicon.md`](../0192-keyboard-on-real-silicon.md) | 430 |
| [`0267-the-tour-is-three-things-wearing-one-name.md`](../0267-the-tour-is-three-things-wearing-one-name.md) | 418 |
| [`0182-x86-64-interactive-boot.md`](../0182-x86-64-interactive-boot.md) | 410 |
| [`0063-name-spellings.md`](../0063-name-spellings.md) | 338 |
| [`0523-the-job-mix-supervisor-in-userspace.md`](../0523-the-job-mix-supervisor-in-userspace.md) | 323 |
| [`0265-proto-is-a-truncation-not-an-abbreviation.md`](../0265-proto-is-a-truncation-not-an-abbreviation.md) | 294 |
| [`0238-the-scheduled-checks-that-never-run.md`](../0238-the-scheduled-checks-that-never-run.md) | 180 |
| [`0263-can-a-timer-be-a-capability.md`](../0263-can-a-timer-be-a-capability.md) | 166 |
| [`0230-shell-check-is-red.md`](../0230-shell-check-is-red.md) | 52 |
| [`0061-caretakers.md`](../0061-caretakers.md) | 15 |
| [`0435-decision-gates-that-name-no-decision.md`](../0435-decision-gates-that-name-no-decision.md) | 6 |

### A note, 155 words over

[`notes/corrections/2026-09-23-the-sweep-that-swept-nothing.md`](../../../notes/corrections/2026-09-23-the-sweep-that-swept-nothing.md).

### Decisions, 4,618 words over

| decision | words over the cap |
|---|---|
| [§86 (whether an NVMe driver can leave the kernel)](../../decisions/0086-el0-nvme-driver.md) | 3,300 |
| [§101 (notification objects)](../../decisions/0101-notification-objects.md) | 466 |
| [§121 (what a device capability is when the device has no page)](../../decisions/0121-port-io-capability.md) | 398 |
| [§27 (the filesystem service)](../../decisions/0027-filesystem-service.md) | 263 |
| [§123 (boot-time re-derivation)](../../decisions/0123-boot-time-rederivation-privilege.md) | 153 |
| [§96 (process kernel or event kernel)](../../decisions/0096-process-kernel-or-event-kernel.md) | 38 |

## How it is done

The method is the one #1992 and #1994 used. A block or note may be tightened as well as split, and
no fact, date or attribution is dropped. A decision is ruled text, so its paragraphs move whole and
nothing is reworded. Its pointer lines say only what each appendix holds, with no figures or claims
(the maintainer's send-back on #1994, under calef's 2026-10-11 ruling on #1919). Every new appendix
meets the §213 (writing standards) limits outright, so a decision paragraph holding a sentence over
40 words stays on the page.

## §86 is blocked

§86 cannot get under the cap by moving paragraphs. Sixteen of them, 2,483 words, hold sentences of 41
to 78 words, so none can go to an appendix. They include the prior-art list and the MSI-X
measurements. The smallest honest page is about 2,863 words, with a median sentence of 21 against
§213's 20. It waits on calef for one of two answers: a marked exception, or leave to split those
sentences as they move, as he allowed for §139 (who may read the cycle counter).

## Done when

Every document listed here is under the cap, or carries a marked exception with its reason, and its
row in `design/prose-baseline.tsv` has fallen to match.
