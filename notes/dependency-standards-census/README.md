# The dependency standards census: data, tools and the proposal

[`notes/dependency-standards-census.md`](../dependency-standards-census.md) is the page to read.
The files here are what it was computed from, and the proposal it leads to.

- [`proposed-decision.md`](proposed-decision.md): one question per nife standard, for an architect.
- `dependencies.tsv`: every third-party crate version in nife's lockfiles, classed as shipped,
  test-image, build-time or host, with the build that links it.
- `census.tsv`: one row per measured crate version, with every measure joined.
- `pass-fail.tsv`: each standard applied to each row.
- `registry.tsv`, `upstream-practices.tsv`, `package-tests.tsv`, `miri-sample.tsv`: the raw tables
  the census joins.
- `tools/`: the scripts. `classify.py` writes `dependencies.tsv` from a nife checkout. `measure.py`
  runs nife's own helpers on a crate's source. `runtests.sh`, `runmiri.sh` and `runmutants.sh` run a
  package's suites in a scratch copy. `join.py` builds `census.tsv` and `pass-fail.tsv`.

Name: provisional, for the directory and every stem in it, minted 2026-10-19 (UTC) by lane
maintainer/dependency-standards-census. The directory follows the `notes/<stem>/` appendix
convention of §212 (a prose budget for every document), so its name is the parent's stem. calef
names directories and files, and `script/names --unratified` lists each stem.
