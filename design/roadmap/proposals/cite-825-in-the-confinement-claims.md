---
status: PROPOSED
raised: 2026-10-15
milestone_dependencies: 825
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# Cite milestone 825's test in the confinement records

*(Title and slug provisional.)*

Milestone 825 (a hostile client races the file server's name window) is BUILT. It added
`redoxfs_server/src/tests/name_window.rs`, which holds the refused-name claim over `Server::handle`.
The confinement records still describe the gap it closed as open.

## The work

- Rows 19 and 24 of `notes/confinement-claims.md` name the window TOCTOU as the open near miss.
  Update them to cite `name_window.rs` and record the held refused-name invariant.
- The refusal log in `notes/confinement-outsider-pass-4.md` lists the race as a declined probe and a
  target for a later pass. Record that 825 closed it defensively.

## Reuse

The edit reuses milestone 825's own `redoxfs_server/src/tests/name_window.rs` as the test to cite,
and amends the existing `notes/confinement-claims.md` rows and pass-4 refusal log in place. No code
is written.

## Why a proposal

These are the confinement claim's own records, which a developer lane does not own. The milestone 825
lane left them to the confinement owner rather than editing them (`design/fatal-risks/7-the-confinement-claim.md`
already carries the dated 2026-10-15 correction that the fatal-risks gate required).
