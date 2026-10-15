# §134 appendix: the patch spellings weighed, and the amendments' evidence

Evidence for [§134 (a harness carries a machine-replayable falsification
record)](../0134-harness-falsification-record.md): the spellings weighed before the unified diff, and what each later
amendment found. The main page is complete without it.

## The mutation spellings weighed

calef, 2026-08-30, choosing between three spellings that differ in what they cost later rather
than now.

- A unified diff in a sibling file. Replay is `git apply`, run the harness, require failure,
  revert. No new tooling at all. Chosen.
- A structured operator record (file, symbol, operator). Survives refactors, and needs a mutation
  engine that can drive `cargo kani`, which `cargo-mutants` was not built for and nobody has tried.
- A `cfg`-gated defect in the source. Cannot rot, because the compiler checks it, and it puts 145
  blocks of deliberately wrong code into shipping source, each a path no normal build exercises.

Why the diff wins, and it is the elegance tenet rather than the convenience one. It has the
fewest moving parts, needs nothing that does not exist, and leaves the operator route open if a
sweep ever proves cheap enough to want it.

Its rot is a feature, and this is the load-bearing claim. A patch that no longer applies means
the covered code moved, which is exactly when a falsification should be redone rather than trusted.
A record that survives a refactor of the thing it falsifies is asserting something nobody checked.

## The patch path, after it was ratified

And one of those two paths has since moved, which is worth leaving visible rather than
correcting in place. Milestone 175 split `user/` into `components/` and `fixtures/` on
2026-09-13, so 197's patch lives at
`components/falsifications/proofs.push_never_writes_past_the_buffer_it_was_given.patch` today and
there is no `user/` package. The clarification above was accurate when it was written; the citation
went stale twelve days later, which is the ordinary cost of naming an instance to illustrate a rule
and is cheaper than not illustrating it.

Amended 2026-08-31, and the first spelling was wrong. It was
`falsifications/<harness_fn_name>.patch`, which assumes a harness function name is unique within its
crate. **In `paging` it is not**: six properties are stated once per ISA across `aarch64.rs`,
`sv39.rs` and `x86_64.rs`, so eighteen harnesses share six names, three would collide on one file,
and `cargo kani --harness index_is_always_in_bounds` cannot separate them either. Milestone 191 (did
the proofs catch the bugs?) had already reported that duplication and the maintainer ratified the
spelling without checking it against the tree. Found by milestone 194's lane on first contact.

## `Expected to fail:` was already in use

**The naming question had already been answered by use, and the proposal that raised it did not
know.** Milestone 323's part 4 was filed as a decision about what to call a new field, on the
observation that *"four patches in the tree already solve this in prose"*. That was a sample. Counted
on 2026-09-18: 66 falsification records, all 66 carrying the line, 65 spelling it exactly
`Expected to fail` and one `Expected red`
(`crates/nifefs/falsifications/verification.the_validation_implies_reads_slice_is_in_bounds.patch`,
corrected in the same change). So the decision was not what to name a field; it was whether to ratify
the name 65 records already used, or rewrite 65 files to no reader's benefit.

## Why "harness" was widened

calef, 2026-09-19, closing a gap `script/falsifications` named against itself and handed back.
Its header says, in the paragraph on milestone 305, that the only word which strains is `harness`,
that it strains in prose rather than in the convention, and that widening the wording is a decisions
edit rather than that script's.
