---
status: REFUSED
raised: 2026-10-14
refused_by: 813
---
# 923. exFAT waits for its patents

Status REFUSED: calef asked on 2026-10-14 (UTC) for exFAT to be checked and blocked on the roadmap
if its patents needed it; they do. *(The number is provisional until the merge queue lands it, and
the title and slug are drafts.)*

Milestone 813 (`file_allocation_table`: a proven FAT crate, released on its own) scopes exFAT for
later, as its own milestone or a sibling crate. This block is that later work, and it is not built
while Microsoft's exFAT patents are in force.

## Why

Microsoft's exFAT patents claim from a 2004-12-17 or 2006-05-23 priority, and four are still live
by Google Patents, checked 2026-10-14 (UTC). Google Patents marks its status an assumption rather
than a legal conclusion, and this record is not legal advice.

| patent | title | anticipated expiry |
|---|---|---|
| [US 8,321,439](https://patents.google.com/patent/US8321439B2/en) | Quick filename lookup using name hash | 2027-03-09 |
| [US 9,122,695](https://patents.google.com/patent/US9122695B2/en) | Extending cluster allocations in an extensible file system | 2027-05-23 |
| [US 10,614,032](https://patents.google.com/patent/US10614032B2/en) | Quick filename lookup using name hash | 2028-01-27 |
| [US 9,639,554](https://patents.google.com/patent/US9639554B2/en) | Extensible file system | 2028-04-14 |

US 9,639,554's first claim reads on the exFAT directory-entry model itself: typed primary and
secondary entries, critical and benign, and refusing to mount a volume with an unrecognized critical
primary entry. An exFAT reader that follows the specification cannot avoid that. The rest of the
family shown on that page (US 8,583,708, 9,336,231, 9,454,542, 9,575,988, 10,303,650 and
10,474,641) is marked expired.

The grant Microsoft made in 2019 does not reach nife. In October 2020 the Open Invention Network
(OIN) added the Linux implementations of exFAT to its Linux System Definition, so the
non-aggression covers OIN licensees using those packages. nife is not Linux, its exFAT would not
be one of those packages, and the nifeos organization is not an OIN licensee.

## Revisit

- **Condition.** 2028-04-14 passes with no later exFAT patent found in force, or a written license
  or covenant that covers nife's own exFAT implementation. Either way, re-check the family on Google
  Patents first, since a term adjustment or a continuation can move the date.

Reuse: hadris-fat 3.0.0-rc.1 (MIT), milestone 813's exFAT reference, is the code to take or learn
from when this is revisited. Taking it now would not help, since the patents read on the format
rather than on any one implementation.

## Index row

exFAT on nife, refused until Microsoft's exFAT patents expire (US 9,639,554, the last known, on
2028-04-14) or a license reaches nife; OIN's grant covers Linux's exFAT, not nife's.
