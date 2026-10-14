# Launch: a pair of outsider passes at the confinement claim

**Name: provisional.** This file turns [the brief](confinement-outsider-pass.md) into one prompt
per pass, for the pair criterion (c) counts: one pass by GLM 5.3 on patagonia, launched by calef,
and one by Claude Fable in Claude's cloud runner, launched by the maintainer. Milestone 920 (an
outsider pass is launched pinned and floor-checked) turns these steps into a launcher; until it
lands they are done by hand.

## The rules both launches keep

- One commit. Both passes attack one main commit, which the lane pins at launch as `sha` below
  and names in each pass's note.
- Concurrent is allowed. Nothing changes between the two passes, so they are consecutive even
  when they run at the same time. Neither can see the other's findings, since neither pushes
  before its report is done. The brief's "never in parallel" is about test runs within one pass.
- Reset. If either finds an escape on a shipped path, the count resets: fix it, then both run
  again at a new pinned commit.
- The prompt holds the reviewer's brief and the runner section below, and nothing else. No
  milestone number, no roadmap block, no pass note, nothing from `lab/`. Both passes are informed,
  so the tree itself is not trimmed.
- Pin the model when starting the session, then record the model id the session reports. If it
  differs from the pin, stop the session before it attacks and start again.

## Assemble the prompts

In the lane's worktree, on a main that holds this file:

```sh
sha=<the pinned commit, full SHA>
scratch="$(mktemp -d -t nife-review)"
reviewer_brief() {
    awk '/^## The reviewer.s brief/{f=1;next} f' briefs/confinement-outsider-pass.md
}
```

### GLM 5.3 on patagonia

The reviewer works in a standalone clone with no remote and one branch, at `sha`, so it cannot
see a main newer than its commit or the other pass's branch.

```sh
git clone --quiet --no-hardlinks "$(git rev-parse --path-format=absolute --git-common-dir)" "$scratch/nife"
git -C "$scratch/nife" remote remove origin
git -C "$scratch/nife" switch --quiet -c pass "$sha"
git -C "$scratch/nife" for-each-ref --format='%(refname:short)' refs/heads |
    grep -vx pass | xargs -n 1 git -C "$scratch/nife" branch --quiet -D
{
    cat <<EOF
## Before you start

This section is about the machine you run on. Where it and the brief below disagree, it wins.

- The checkout is \`$scratch/nife\`, on branch \`pass\` at commit \`$sha\`. Stay on that branch
  and that commit. It has no remote; do not add one.
- This is the maintainers' Mac, and other work may be loading it. Before each \`script/test\` leg,
  record \`uptime\`. A timeout is inconclusive, not a hold or an escape: rerun that leg once, alone,
  and report both runs with their load.
- End every commit with \`Co-Authored-By: GLM 5.3 <noreply@z.ai>\`.

EOF
    reviewer_brief | sed -e "s|<path>|$scratch/nife/notes/confinement-claims.md, under \"## The claims\"|" \
        -e "s|<worktree>|$scratch/nife|" -e "s|<results path>|$scratch/results.md|"
} > "$scratch/glm-prompt.md"
echo "$scratch"
```

calef's one prompt: start GLM 5.3's harness (opencode) with the model pinned, in `$scratch/nife`,
and say "Read `$scratch/glm-prompt.md` and do it." Note the session id: the completion round
resumes it.

### Claude Fable in Claude's cloud runner

The runner shares no file system with the lane, and the report comes back as a committed file on
a pushed branch. A runner new to this tree gets a setup-only dry run first: send the "Setup" steps
below alone and record what they print.

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
- Your report is a file at the root of the repository, \`pass-results.md\`, committed
  last. Then push this branch once. Push nothing before the report is finished, and open no pull
  request.
- End every commit with \`Co-Authored-By:\` and the model this session reports, with
  \`<noreply@anthropic.com>\`.

EOF
    reviewer_brief | sed -e 's|at `<path>`|in `notes/confinement-claims.md`, under "## The claims"|' \
        -e 's|`<worktree>`|the repository this session started in|' \
        -e 's|`<results path>`, outside the repository|`pass-results.md`, as above|'
} > "$scratch/fable-prompt.md"
```

The maintainer's one prompt: start a cloud agent with the model set to Fable and the repository
`nifeos/nife`, and give it the contents of `$scratch/fable-prompt.md` as its task. Read the served
model from the session's record (`last_served_model`) before counting the pass.

## The lane, for each pass

Brief, collect, reproduce, record; never attack and never hint.

- Pin `sha`, assemble the prompt, and hand it to whoever launches. Read the prompt once before
  handing it over: it holds the brief and the section above, and nothing else.
- Collect: the GLM pass's commits from `$scratch/nife` (`git fetch "$scratch/nife" pass`) and its
  `results.md`; the Fable pass's pushed branch and its report.
- Reproduce every booted row's test on patagonia, one ISA's QEMU at a time, at `sha`. A timeout
  under load is inconclusive; rerun it on an idle machine before recording it.
- Reproduce each claimed escape by test or boot before recording it, and commit its red test
  before any fix.
- Count booted rows against the floor. Short, send the completion round the brief describes, in
  the same session, once, and count again.
- Write the pass note in pass 7's format, with `sha`, the model reported, the host facts
  (patagonia's load, or the runner's `uname -m`, `nproc` and leg times), whether a completion round
  ran, and the refusal log.
