# Prior art, read 2026-10-12

An appendix of [nife's tooling, sorted by who needs it](../0902-tooling-sorted-by-who-needs-it.md),
moved out of the block on 2026-10-12 (UTC) when its promotion took the block to the 3,000-word cap
of §212 (a prose budget). How four large projects divide in-tree standards from the workflow
automation that lives outside the tree:

| Project | In-tree standards | Workflow automation, outside the tree |
|---|---|---|
| Rust | [`src/tools/tidy`](https://github.com/rust-lang/rust/tree/main/src/tools/tidy): "a custom tool used for validating source code style and formatting conventions" ([rustc-dev-guide](https://rustc-dev-guide.rust-lang.org/tests/intro.html)) | [triagebot](https://github.com/rust-lang/triagebot), "the triage and team assistance bot", and [bors](https://github.com/rust-lang/bors), the merge bot, each its own repository |
| Linux | `scripts/checkpatch.pl`, run before submitting ([submitting-patches](https://docs.kernel.org/process/submitting-patches.html)) | [b4](https://github.com/mricon/b4) (canonical repo on git.kernel.org) for patch workflows, and [patchwork](https://github.com/getpatchwork/patchwork), "a patch tracking system" |
| Chromium | `PRESUBMIT.py` files in the source tree, found by walking up from each changed file ([presubmit scripts](https://www.chromium.org/developers/how-tos/depottools/presubmit-scripts/)) | [depot_tools](https://commondatastorage.googleapis.com/chrome-infra-docs/flat/depot_tools/docs/html/depot_tools_tutorial.html), one repository of "git workflow-enhancing tools" whose `git cl` runs those files |
| Kubernetes | `hack/verify-*.sh` ([hack/](https://github.com/kubernetes/kubernetes/tree/master/hack)) | [Prow](https://github.com/kubernetes-sigs/prow), with job configs in [test-infra](https://github.com/kubernetes/test-infra) |
