---
status: BUILT
raised: 2026-09-19
built: 2026-10-15
promoted_from: a-committed-blob-that-matches-its-source
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 491. A committed `.dtb` is checked against what its `.dts` compiles to

*(Number provisional until the merge queue lands it.)* Promoted from the
proposal `a-committed-blob-that-matches-its-source`, filed 2026-09-19, on calef's instruction of
2026-09-20 to give every proposal on `main` a number. The argument is the proposal's. Found by
milestone 326 (nobody has been assigned to turn a)'s `machine_discovery` lane while regenerating a fixture for an unrelated
reason, and filed by the integrator at merge because the lane's report was its only home.

The check is a `script/lint` entry over files already in the tree.

**In brief.** `script/lint` compiles every committed `.dts` with `dtc` and compares the result
against the committed `.dtb` beside it: same stem, same directory. The comparison is the
decompiled structure of the two blobs, not their bytes. A `dtc` version difference therefore
cannot fail a contributor. Where `dtc` is absent the check skips, and its output says so: a skip
is a pass on that machine, and CI's lint job installs `dtc`, so the gate is real where the merge
queue runs. Five blobs once said `cricker,` in their root `compatible` five weeks after the
2026-08-15 rename. A sweep can `sed` text and cannot `sed` a binary, and no test read the
property. Milestone 326's lane regenerated all five, commit `38cc4c1`, and fixed six stale
`crates/isa/tests/fixtures/` paths in the same headers. What stayed open was the mechanism; this
check is the mechanism. When it landed, on 2026-10-15 (UTC), the comparison covered 31 pairs.
Seven further committed `.dtb` blobs, QEMU's own dumps, have no source beside them.

## Why this is worth a check rather than a habit

A committed artifact that nothing regenerates is a fact nobody re-reads. That is the shape this
tree keeps finding: a `BUGS` entry nobody struck when the work landed, a baseline column six weeks
stale being read as last week's census, a roadmap status that was wrong in both records. A blob is
the worst member of that family because no reader can see it. A stale sentence at least shows up
in a diff.

It is also the failure the rename convention cannot reach by construction. `design/naming.md`
describes how a ratified rename is performed, and every step of it operates on text.

## The two shapes

Rung two, built: the `script/lint` check described above, in
`helpers/lint_dtb_matches_dts.py` and its `--selftest`. It fires without anybody remembering,
like every other check in `script/lint`.

Rung one, refused: drop the blobs and compile at build time. That would make fixtures vary with the
local `dtc`, which is not byte-stable across versions, so the test input itself would stop being
stable unless `dtc` were pinned too. It also puts a build dependency on `dtc` on every contributor
and every CI leg, a cost every builder pays under DECISIONS §46. The one incident on record is
already closed at every push by the check above. Reopens on a second drift incident, or on drift in
the sourceless fixtures.

## BUGS

- A skip is a pass on a machine without `dtc`. The check's own output says so, and names CI's
  lint job, which installs the tool. The residue is real: a contributor whose laptop lacks `dtc`
  gets no local proof, only the note.
- **It does not generalize to other committed binaries.** The check is a comparison, so it covers
  the one case where a comparison is possible: a committed `.dtb` beside its committed `.dts`.
  `bench/` images, the font atlases and the RedoxFS test image have the same property and no
  source beside them. The seven QEMU-dumped `.dtb` fixtures are in the same position. The check
  prints their count rather than passing over them in silence.
- The selftest drives a stub `dtc`. It proves the plumbing: match, mismatch, skip, the zero-pair
  guard, the coverage counts. It does not prove real device-tree parsing. The parsing is proved
  on the tree's own fixtures, on every lint run.

## Follow-on

- **Refused.** Rung one, compiling the blobs at build time; the reason and the reopening
  condition are under "The two shapes".

## Index row

`script/lint` compiles every committed `.dts` and compares it against the committed `.dtb`
beside it. The comparison is decompiled structure, so a device-tree fixture cannot drift from
its source the way a text sweep leaves a binary behind. A skip on a machine without `dtc` is a
pass, and the output says so, with CI named as the place the gate is real. Blobs with no source
beside them are counted in the check's output and not compared.
