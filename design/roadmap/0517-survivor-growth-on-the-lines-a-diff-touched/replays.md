# Does the blame proxy tell the truth: the six replays

An appendix to [milestone 517 (what fraction of survivor growth arrives on lines a pull request
touched)](../0517-survivor-growth-on-the-lines-a-diff-touched.md). The main page is complete without
it. This is the replay of `cargo mutants --in-diff` against six commits, which checked that blame
attributes each survivor to a commit whose own diff-scoped run would have reported it.

## The replays

Each replay is `cargo mutants --in-diff` against that commit's own diff, inside a `git archive` of
the commit, tool pinned at 27.1.0, the same shape milestone 438 used. The question is not how many
survivors the gate reports. It is whether the survivors blame attributes to that commit are among
them.

| replay | commit | survivors blame attributes | reported by `--in-diff` | mutants, wall clock |
|---|---|---|---|---|
| #207, `mdns_proto` | `6bc7751e` | 76 | 76 (of 82 reported) | 373 in 2m |
| #326, `timetable` | `0c61d6af` | 48 | 48 (of 50) | 172 in 50s |
| `video_terminal` | `1b60e047` | 66 | 66 (of 66) | 120 in 48s |
| a clippy fix | `858a1268` | 1 | 1 (of 1) | 7 in 15s |
| a lint pass on `pci` | `d6ad03dd` | 1 | 1 (of 1) | 15 in 6s |
| milestone 231 (nothing counts how many capability slots a boot actually uses) | `34b52ac3` | 5 | 5 (of 22) | 32 in 18s |
| total | | 197 | 197 | |

Not one attributed survivor was missing from the replay. 197 is 31% of the 629, chosen to include
both the concentrated case and the tail. The two smallest samples were picked *because* they are
the shape that should have broken the proxy: commits whose message is a lint or clippy fix, where
blame would plausibly be pointing at a cosmetic rewrite of an older line. Both reported their
attributed survivor.

## The miss that was a rename

One of them nearly read as a failure and the reason is worth keeping. `858a1268` reported
`crates/isa/src/plic.rs: replace < with <= in PlicContexts::from_device_tree`. The census names
`crates/machine_discovery/src/plic.rs` for the same mutant: by the window's end the crate `isa` is
`machine_discovery`.
 A comparison keyed on the path scores that as a miss.
Mutants have to be matched on the function and the mutation, not on where the file lived that week.

## Why the replays report more than blame attributes

The replays also report more than blame attributes, 82 against 76 and 22 against 5. A pull
request's diff contains lines other commits wrote, and the gate mutates all of them. That direction
costs a contributor work and does not change this fraction.
