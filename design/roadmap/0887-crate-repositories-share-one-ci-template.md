---
status: PARTIAL
raised: 2026-10-11
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: yes
---
# 887. Crate repositories share one CI template, adapted from nife's gates

*(The title, the slug and the template repository's name are provisional, for calef's naming
pass.)*

calef, 2026-10-11 (UTC): *"As a gate for each of these milestones, I want to ensure that we're
exposing clean crates without nife assumptions. I also want to ensure that the gates that we have
for nife will be adapted for these crate repos as well."* Approving this block's plan:
*"Yes. Don't we want proofs and falsifications for our crates?"*

857's Fork 1 already ruled the shape on 2026-10-07: one repository per crate under `nifeos`. Its
words: *"Every crate repository uses one shared CI template (Kani, lint, release) through reusable
workflows."*

Fork 4 ruled the proof job (`kani-github-action`), plus a job that applies each falsification
patch and expects Kani to fail. It also ruled that the release refuses to publish unless the
proofs are green. Fork 6 ruled trusted publishing from a `nifeos` workflow. Until this block, no milestone
built any of it, and every per-crate block from 813 to 821 said so in its `BUGS`.

## What this builds

1. A template repository, provisionally `nifeos/crate-workflows`, holding reusable workflows
   (`on: workflow_call`), one per row group below: `check`, `proofs`, `weekly` and `release`. A
   crate repository's own workflow files are a few lines each that call them, pinned by commit SHA,
   as nife pins every action today.
2. Versioned releases of the template (`v1`, `v2`, ...), each a tag on one commit.
3. A lag check, in the template itself so no crate repository can forget it. Every reusable
   workflow's first job compares the SHA it was called at (`job_workflow_sha`) with the template's
   newest release and fails when it is older. A weekly schedule in each crate repository runs the
   same check with no pull request open, and Dependabot's `github-actions` ecosystem opens the bump.
4. The release machinery 857 ruled and nobody owns yet. In nife, a lint makes `publish = false`
   the default and allows publishing only for crates on calef's allowlist (Fork 3). The list is
   one file, provisionally `crates/publish-allowlist.txt`, which `script/metrics` already reads. Outside it, the
   `nifeos` crates.io team and the trusted publisher configuration (Fork 6). The `release`
   workflow publishes only from a tag, only after `check` and `proofs` pass on that commit, and only
   through a GitHub environment calef approves.
5. Per-repository CI time. `helpers/ci_job_times.py` reads one repository's merge-group runs today;
   it takes a repository argument and reads each crate repository too, so a slow template job shows
   up in nife's metrics rather than in nine places nobody watches.

## Repository configuration

calef, 2026-10-11 (UTC): *"nife is MIT OR Apache-2.0, so we should dual license our crates as well.
I agree with your recommendations on the crate repos."* The recommendations he agreed to are below.
They live in the template as one settings file and a script that applies it with `gh api`. A
weekly drift check in each crate repository reads the live settings back and fails on any
difference. A setting changed by hand in the web interface is then a red run, not a surprise.

- Features: issues on, with issue forms for a bug report and a feature request and blank issues
  off. Wiki, projects, discussions and sponsorships off; discussions turn on per crate only if
  questions crowd the issues.
- Merging: merge commits only, no merge queue at this traffic, branches required up to date
  (strict status checks), and auto-merge, update branch and delete-on-merge all on.
- A ruleset on `main`: no deletion, no force push, a pull request required with conversations
  resolved and zero approvals. The required checks are the template's jobs: fmt, clippy, tests,
  Kani, falsification replay, coverage, `crate-standalone`, supply chain and the architect hold.
  calef agreed to keep `needs-architect` as a required hold, so a semver-breaking API change waits
  for him: *"We should keep the needs-architect hold on the crates."* A hold nobody can trigger
  holds nothing. The template's label workflow therefore adds the label itself, on two rules: a
  dependency new to the crate's graph, and a breaking change that `cargo-semver-checks` reports
  against the last published version.
- A tag ruleset on `v*`, in each repository: no tag is updated or deleted, and creating one is
  refused to everyone except the `nife-smelter` app, the only bypass actor, which the release
  workflow's tag job authenticates as. A hand-made `v*` tag is refused. The template's
  `script/apply-settings` applies it from the settings file.
- A `crates-io` environment with calef as required reviewer, and the trusted publisher bound to
  it. Fork 6's approval is then enforced by GitHub rather than remembered.
- Security: private vulnerability reporting and a `SECURITY.md`, secret scanning with push
  protection, and Dependabot alerts, security updates and version updates for `cargo` and
  `github-actions`.
- Actions: only SHA-pinned actions from GitHub, verified creators and an allowlist naming
  `taiki-e/install-action@*`, the template's one third-party action (named in
  `settings/repository-settings.json`). The default `GITHUB_TOKEN` is read-only, and the release job alone gets
  `id-token: write`. A fork's pull request runs no workflow until a maintainer approves it, for
  every outside contributor and every time. That is the `all_external_contributors` approval
  policy nife has had since milestone 588 (a merge needs a review no fork can supply), not
  GitHub's default of first-time contributors only. calef asked for it here on 2026-10-12 (UTC).
  The settings script sets it on each repository rather than trusting an organization default,
  and the drift check reads it back.
- Files: a README, and `LICENSE-MIT` and `LICENSE-APACHE` with `license = "MIT OR Apache-2.0"`
  as in nife's workspace manifest. Then `SECURITY.md`, `CHANGELOG.md`, and `CONTRIBUTING.md`, which
  says our work is disclosed as agent-written (Fork 8) and outside contributions are welcome. Then
  `CODEOWNERS` naming the `nifeos` team, the issue forms, a pull request template, and a small label
  set synced from the template, `needs-architect` among it.
- Access: a `nifeos` crates team with Maintain, calef as admin, and the `nife-smelter` app with
  Write.
- Metadata: a one-line description, docs.rs as the homepage, and the topics `rust`, `no-std`,
  `formal-verification` and `kani`, plus the crate's domain.

Both license files, because GitHub's license detection reads nife itself as Apache-2.0 although the
tree carries both files and the workspace says `MIT OR Apache-2.0`. `script/crate-standalone`
checks the pair and the field while a crate is still here.

## Each nife gate, kept, adapted or dropped

Read 2026-10-11 from `script/ci-build`, `script/lint`, `.github/workflows/` and the helpers they
call. A crate repository is a `no_std` library with host tests and Kani proofs, and no kernel,
QEMU, roadmap or merge queue, so that is the test each row applies.

| nife gate | crate repository | reason |
|---|---|---|
| `script/fmt` (rustfmt) | adapt | Stable rustfmt without nife's two unstable options, since the crate builds on stable |
| clippy `-D warnings` | adapt | Host, each listed target and a `--cfg kani` build, at the stated MSRV and at the newest stable |
| rustdoc broken links | keep | A published crate's docs are its manual on docs.rs |
| cargo-machete | keep | An unused dependency is a cost every downstream user pays |
| host tests (`cargo test`) | keep | No QEMU leg; the crates are pure logic |
| coverage, 85% per file | keep | Same floor, same tool (cargo-llvm-cov); the block's 80 predates the raise to 85 |
| fuzz (cargo-fuzz) | keep | For crates that parse bytes; nightly, so weekly rather than on the publish path |
| Miri, weekly | keep | Same job, nightly |
| mutation (cargo-mutants), weekly | keep | A report, as in nife |
| `script/verify` (Kani) | adapt | Kani installed with `cargo install` at the version in `.kani-version` (0.68.0), run on every pull request (Fork 4) |
| falsification sweep | adapt | Every patch applied with `-p1` must turn its harness red at its own assertion; on every pull request, not weekly, since a crate has few |
| `script/falsifications --check` | keep | Every harness carries a patch or a stated reason |
| kani-reach | drop | A dispatch-only measurement of nife's reach |
| loom interleavings | drop | None of the nine crates uses loom; return when one does |
| cargo-deny | adapt | nife's policy without its own exceptions (`rayon`, the `rsa` advisory, the vendored tree) |
| vendor-verify, vendor-watch | drop | No vendored code |
| toolchain drift and bump | adapt | A weekly build on the newest stable and beta replaces the nightly pin |
| MSRV build | new | The stated `rust-version` builds; nife has no MSRV to check |
| semver check | adapt | cargo-semver-checks against the last release stands in for milestone 610 (the interface's stability is measured weekly) |
| `script/crate-standalone` | adapt | Its citation and file checks run in the repository, so nife's vocabulary cannot creep back |
| file-length ratchet, 2,000 lines | keep | The same limit on Rust files |
| comment-block ratchet | keep | Same rule |
| workspace lints (`unsafe_op_in_unsafe_fn`, `missing_docs`) | keep | Inlined into each crate's own `[lints]`, which `crate-standalone` requires anyway |
| orphan Rust files | keep | Every `.rs` file is in the module tree |
| prose ratchet | adapt | The README and changelog, at nife's caps |
| house style, typos, no em-dashes | keep | American spelling and the same `typos` run |
| markdown links resolve | keep | Same check |
| EXAMPLES and BUGS | new | nife has no mechanical gate for its own documentation standard. The template checks that the README carries both headings |
| actions pinned to a SHA | keep | Supply chain, unchanged |
| a workflow that swallows a failure says where it goes | keep | Same rule |
| shellcheck, `sh -n` | keep | Where a repository has scripts |
| tracked-but-ignored files | keep | Same check |
| job budgets | adapt | Per repository, with time read by `helpers/ci_job_times.py` |
| CodeQL (Rust) | keep | Same job |
| commit-msg `Co-Authored-By` hook | keep | Agent attribution is the rule everywhere nife writes |
| empty diff | keep | Same check |
| architect label (`helpers/architect-label-rules.py`, `architect-label.yml`) | adapt | Keeps the rule for a dependency new to the crate's graph and adds a semver rule (below). Drops the abi-surface, spawnproto, syscall-error and decisions rules unless a crate carries the matching files |
| architect hold (`architect-hold.yml`) | keep | Red while `needs-architect` is on, and a required check |
| `script/record-ruling` | adapt | A ruling swaps `needs-architect` for `architect-ruled` the same way, from a template script |
| naming checks 1 to 3 (`-d`, daemon, contract spelling) | drop | nife's own vocabulary |
| name provenance blocks (`script/names`) | drop | A crate's name is ratified before first publish; a `Name:` block is a nife record `crate-standalone` refuses |
| citations, decisions, roadmap, fatal risks, audits index, TODO cites a milestone | drop | Records that live in nife. A TODO in a crate repository cites an issue there instead |
| branch names, ready-status, script docs, env names in prose, agent instructions | drop | nife's lane and document process |
| merge drain, trunk health, ci-failing, verify shard refresh | drop | No merge queue at one crate's traffic; return when one has a queue |
| audit and stranger cadence | adapt | A stranger builds and proves the crate from its README alone, which is the release's whole claim |
| kernel and image gates: boot, swish, icount, bench, cpu-matrix, stack frames, W^X, reproducible image, arch rules, syscall registers, `#[path]` sharing | drop | Kernel, QEMU or image properties; a library crate has none |

## Reuse

Taken, not written: GitHub's reusable workflows, `rust-lang/crates-io-auth-action` for trusted
publishing (RFC 3691), Dependabot, `cargo-semver-checks`, and `taiki-e/install-action` for the
tools nife already pins. Kani is the exception to taking an action: the template installs it with
`cargo install` at the version in `.kani-version`, so the Actions allowlist names no Kani action. A
weekly template job proposes the version bump as one pull request (crate-workflows #5). Prior art for
the shape: rust-embedded's and RustCrypto's shared workflows, read for structure rather than
copied. The workflows themselves are written, since none publishes a falsification job.

## Exit

A crate repository, `nifeos/crate-workflows-test`, calls every reusable workflow from releases v1
and v2. Built 2026-10-16 (UTC). The same day, calef ruled that it
stands as the template's release canary, adopting every new release before a crate depends on
one. It has adopted release v5 (commit 30d26230faf82f54653cb302803edf2207f5392b), through its pull
request 5, merged at 726058495831c9bd3968c405a4d5354709e3233e on `main`, which is the ritual's
first run. The template's
own selftest is green on every job: check, proofs, and weekly against `sample/`, plus the release gate's refusal. The canary's
earlier pull requests show the rows red on a commit made to break them and green on the
commit after: fmt, tests, coverage, standalone and the falsification replay on one pair. Another
pair shows the lag check naming a pin behind the newest release, and the supply-chain refusal of a
floating action. A pull request held by `needs-architect` took a recorded ruling, which swapped
the label for `architect-ruled` and turned the hold green. The false positive behind it was a
defect in the label workflow's rule 2, fixed in v2. The apply script configures the canary
from the settings file and reports what it cannot do. It sets the features, the security toggles,
the Actions permissions, the `crates-io` environment with calef as required reviewer, the team,
the labels, and the `main` ruleset with the nine required checks. The drift check reads the
settings back. It went red naming a hand-changed setting. The tag ruleset is live on the canary
(ruleset 25290943), and `script/apply-settings` applies it from the settings file. The release gate passed on a green
commit and created no tag under `dry-run`. The publish job then sat in the `crates-io`
environment waiting on a person, which is the "publishes nothing without calef's environment
approval" row. In nife, the allowlist lint fails a crate that drops `publish = false` without
being on the list, and `helpers/ci_job_times.py` reads a crate repository by argument.

## BUGS

- The table is a reading of nife's gates on 2026-10-11, not a ruling. A gate added to nife
  after that date is not in it, and nothing yet makes adding one ask "and the template?".
- The template's repository name (`crate-workflows`), workflow names and release cadence are
  provisional. The `crates-io` team slug and the required-check names (`ci / fmt`, ...) are
  provisional with them; the check names are measured against what GitHub shows for
  cross-repository reusable workflow calls and match.
- `needs_person` is `yes`. The crates.io trusted publisher needs calef's crates.io account, and
  the fork pull request approval policy is an organization administration setting, which the
  lane's token does not carry.
- A crates.io trusted publisher cannot be bound before a crate's first publish. crates.io's
  trusted-publishing documentation (rust-lang/crates.io,
  `svelte/src/routes/docs/trusted-publishing/+page.svelte`) says: "Your crate must already be
  published to crates.io (initial publish requires an API token)." How a crate's first publish
  happens is an open question for calef; this block does not answer it.
- A push-triggered cross-repository reusable workflow call start-fails on GitHub with an empty
  error (crate-workflows-test runs 38914536439 and 38914537601, same file green on
  pull_request), so the gating callers trigger on pull_request only. A release dispatches at a
  merge commit, whose runs exist from the pull request event, so nothing is lost.
- The semver half of the labeler never fired against a real baseline: the test crate is
  unpublished, so `cargo-semver-checks` has nothing to compare. It is exercised only to the
  no-baseline path. The first published crate is the test, and its publication waits on that crate's
  soak (milestone 818 (`jh7110_entropy`: proven JH7110 TRNG logic, released on its own) is the pilot).
- The settings-drift check's security rows and its ruleset read want an administration-scoped
  token; with the caller's `GITHUB_TOKEN` they report what was unreadable rather than passing
  silently.

## Follow-on

- **Outstanding.** A real crates.io publish end to end: a trusted publisher bound on calef's
  crates.io account, a tag made by the release workflow, and the publish itself. The bind needs
  the crate to be published already (see `BUGS`). Checked by the dry-run rehearsal, which stopped
  at the environment's approval gate by design.
- **Outstanding.** The labeler's semver rule firing against a published baseline. The test crate
  is unpublished, so only the no-baseline path has run; the false positive it produced is fixed in
  v2. The first published crate (milestones 813 to 821) is the test.
- **Outstanding.** A driver for the canary's release ritual. After each template release, a pull
  request on `nifeos/crate-workflows-test` moves every caller's pin to the new release's commit.
  A template-side workflow can open it once `nife-smelter` holds the workflows permission, since
  the pull request edits `.github/workflows`; the app holds contents and pull requests write,
  measured 2026-10-17 (UTC). Until then it is by hand.
- **Outstanding.** The fork pull request approval policy. `nifeos/crate-workflows` and
  `nifeos/crate-workflows-test` both read `first_time_contributors`, measured 2026-10-17 (UTC);
  calef asked for `all_external_contributors`. Checked by reading
  `actions/permissions/fork-pr-contributor-approval` on each repository.
- **Outstanding.** The stranger cadence's first run, a person's as in nife. `docs/stranger-test.md`
  records none, so the weekly job fails. Checked by that job going green.
- **Outstanding.** calef's naming pass over the names this milestone shipped provisionally: the
  repository `crate-workflows`, the canary repository's name, the workflow names, the `crates-io`
  team slug, and the required-check names. GitHub redirects renames, and required-check names are
  fixed per ruleset at adoption, so renaming after the first crate repositories exist costs one
  ruleset update each.

## Index row

One shared CI template for every crate repository nife releases, as reusable workflows adapted
gate by gate from nife's own. A check fails when a repository lags the template, and the template
carries the release machinery 857 ruled: the allowlist lint, the crates.io team and trusted
publishing.
