---
status: NOT-STARTED
promoted_from: point-script-drift-at-the-host-excluded-derivation
raised: 2026-10-16
milestone_dependencies: 393
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 931. Point script/drift's host exclusions at the derived set

*(Title and slug provisional.)*

Milestone 393 (four files each record which crates cannot compile for the host, and one of them
derives it) derived the host-excluded crate set once, in `helpers/host-excluded-crates.sh`. Four
consumers are wired to it: `script/lint`'s clippy passes, `script/coverage`, `script/mutation`,
and the x86_64 test leg. `script/drift`'s `cargo test` line still hand-writes its own exclusions
(`kernel`, `system_tests`, `components`, `fixtures`) beside its own `BUGS` entry. The entry records
why it stayed hand-written: a crate missing from that hand list fails the line loudly, which is a
gated defect rather than a silent one. That reason was sound before one derivation served the
tree; now the derivation is the record and a fifth consumer repeating a fourth list is the drift
393 closed.

## The work

- `script/drift`'s `cargo test` line consumes `helpers/host-excluded-crates.sh` instead of
  restating the set.
- The `BUGS` entry retires, its reason recorded in the commit that removes it.

## Reuse

The helper is the reuse. Nothing new is written; the line changes its source of truth.

## Index row

`script/drift`'s `cargo test` line still hand-writes the host-excluded crate set that milestone 393 derived once in `helpers/host-excluded-crates.sh`. It should consume the derivation, and its `BUGS` entry retires.
