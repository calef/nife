# Launch: outsider passes 8 and 9 at the confinement claim

Its name is provisional. Written 2026-10-13 (UTC) by lane `maintainer/outsider-passes-eight-and-nine`
on calef's ruling recorded in milestone 913 (an eighth outsider pass attacks the confinement claim).
Pass 8 is 913, GLM 5.3, launched by calef on patagonia. Pass 9 is milestone 914 (a
ninth outsider pass attacks the confinement claim), Claude Fable in Claude's cloud runner, launched
by the maintainer. This file turns [the brief](confinement-outsider-pass.md) into one prompt per
pass. The reviewer never sees this file: its name contains `outsider`, which the blind posture
forbids.

## The rules both launches keep

- One commit. Both passes attack the first main commit after #2035 lands: the merge commit
  that brings milestone 824 (a retype mints no right its budget lacks) to main. The lane pins it
  at launch as `sha` below and names it in each pass's note.
- Concurrent is allowed. Nothing changes between the two passes, so they are consecutive even
  when they run at the same time, and running them together keeps each blind to the other. The
  brief's "never in parallel" is about test runs within one pass.
- Reset. If either finds an escape on a shipped path, the count resets: fix it, then both run
  again at a new pinned commit.
- Nothing from earlier passes, and nothing from `lab/`. The prompt holds the reviewer's brief,
  the claims table from `sha`, and the variant section below. No pass note, no finding, no
  machine-local fact.

## Assemble the prompts

In the lane's worktree, on a main that holds this file:

```sh
sha=<the pinned commit, full SHA>
scratch="$(mktemp -d -t nife-review)"
table() {
    git show "$sha:notes/confinement-claims.md" |
        awk '/^## The claims/{f=1} f{print} f&&/^\|/{t=1} t&&!/^\|/{exit}'
}
# The reviewer's brief with the Blind posture only (the brief says to fill the posture in).
reviewer_brief() {
    awk '/^## The reviewer.s brief/{f=1;next} f' briefs/confinement-outsider-pass.md |
        awk '/^Informed:/{s=1} s&&/^$/{s=0} !s'
}
```

### Pass 8 (913): GLM 5.3 on patagonia

The reviewer works in a standalone clone with no remote and one branch, so it cannot see
another lane's branch, a main newer than `sha`, or pass 9's pushed results. Its history before
`sha` is still there; the brief forbids reading it.

```sh
git clone --quiet --no-hardlinks "$(git rev-parse --path-format=absolute --git-common-dir)" "$scratch/nife"
git -C "$scratch/nife" remote remove origin
git -C "$scratch/nife" switch --quiet -c pass-8 "$sha"
git -C "$scratch/nife" for-each-ref --format='%(refname:short)' refs/heads |
    grep -vx pass-8 | xargs -n 1 git -C "$scratch/nife" branch --quiet -D
table > "$scratch/claims-table.md"
{
    cat <<EOF
## Before you start

This section is about the machine you run on. Where it and the brief below disagree, it wins.

- The checkout is \`$scratch/nife\`, on branch \`pass-8\` at commit \`$sha\`. Stay on that branch
  and that commit. It has no remote; do not add one.
- This is the maintainers' Mac, and other work may be loading it. Before each \`script/test\` leg,
  record \`uptime\`. A timeout is inconclusive, not a hold or an escape: rerun that leg once, alone,
  and report both runs with their load.
- End every commit with \`Co-Authored-By: GLM 5.3 <noreply@z.ai>\`.

EOF
    reviewer_brief | sed -e "s|<path>|$scratch/claims-table.md|" -e "s|<worktree>|$scratch/nife|" \
        -e "s|<results path>|$scratch/results.md|"
} > "$scratch/pass-8-prompt.md"
echo "$scratch"
```

calef's one prompt: start GLM 5.3's harness (opencode, as for pass 4) in `$scratch/nife` and say
"Read `$scratch/pass-8-prompt.md` and do it."

### Pass 9 (914): Claude Fable in Claude's cloud runner

The runner shares no file system with the lane, so the table goes inline, and the report comes
back as a committed file on a pushed branch. Run the setup-only dry run that 914 requires first:
send the "Setup" steps below alone and record what they print. 914's block holds that run. Its
`apt-get update` line is there because the runner starts with empty apt lists, and `script/ci-qemu`
at the pinned commit installs a package before it updates them.

```sh
{
    cat <<EOF
## Before you start

You are in Claude's cloud runner, not on the maintainers' Mac. Where this section and the brief
below disagree, this section wins.

### Setup

Run these in the repository, in order. Record each command's output tail and wall time in your
report. If one fails, stop and report it; do not change the tree to get past it.

    git reset --hard $sha
    apt-get update
    script/ci-qemu
    NIFE_SKIP_LINT_TOOLS=1 script/bootstrap
    script/qemu-check
    uname -m; nproc; df -h .
    script/test --arch aarch64 --test seventh_pass

### Here, not on the Mac

- Work at commit \`$sha\`, on the branch this session started on. Do not fetch other branches.
- Skip the brief's \`/opt/homebrew/...\` \`PATH\` line; it is for macOS.
- Every \`script/test\` leg runs under TCG. Record each leg's wall time. A timeout is
  inconclusive, not a hold or an escape: rerun that leg once, alone, and report both runs.
- Your report is a file at the root of the repository, \`pass-9-results.md\`, committed
  last. Then push this branch once. Push nothing before the report is finished, and open no pull
  request.
- End every commit with \`Co-Authored-By: Claude Fable <noreply@anthropic.com>\`.

EOF
    reviewer_brief | sed -e 's|at `<path>`|at the end of this prompt|' \
        -e 's|`<worktree>`|the repository this session started in|' \
        -e 's|`<results path>`, outside the repository|`pass-9-results.md`, as above|'
    printf '\n## The claims table\n\n'
    table
} > "$scratch/pass-9-prompt.md"
```

The maintainer's one prompt: start a cloud agent on Fable with the repository `nifeos/nife`, and
give it the contents of `$scratch/pass-9-prompt.md` as its task. 914's `BUGS` lists what the runner
may lack.

## The lane, for each pass

Brief, collect, reproduce, record; never attack and never hint. Per 913 and 914:

- Pin `sha`, assemble the prompt, and hand it to whoever launches. Read the prompt once before
  handing it over: it holds the brief, the table and the section above, and nothing else.
- Collect: pass 8's commits from `$scratch/nife` (`git fetch "$scratch/nife" pass-8`) and its
  `results.md`; pass 9's pushed branch and its report.
- Reproduce every booted row's test on patagonia, one ISA's QEMU at a time, at `sha`. A timeout
  under load is inconclusive; rerun it on an idle machine before recording it.
- Reproduce each claimed escape by test or boot before recording it, and commit its red test
  before any fix.
- Count booted rows against the floor, then write the pass note in pass 7's format, with `sha`,
  the model, the host facts (patagonia's load, or the runner's `uname -m`, `nproc` and leg
  times) and the refusal log.
