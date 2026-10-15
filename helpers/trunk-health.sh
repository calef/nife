#!/bin/sh
#
# Say when `main` is red, and say when it recovers.
#
#     helpers/trunk-health.sh             # watch until stopped
#     helpers/trunk-health.sh --once      # print the current state and exit
#
# PROVISIONAL NAME. Minted 2026-08-04; not put to calef. See the `Name:` block below.
#
# # Why this exists
#
# Nothing owned trunk health, and the gap was in the role definitions rather than in anyone's
# execution. A developer works one milestone in one lane and cannot see `main` by design. The
# steward watches pull request checks, conflicts, at-risk work and idle lanes, and its charter never
# mentioned the trunk. The maintainer is told to keep hygiene, and that list is prune the worktree,
# delete the branch, relink `nife-dev`, leave no QEMU. So the role that merges is the role that
# breaks `main`, and the role that exists to compensate for the merger being busy was not pointed at
# the thing merging breaks.
#
# **The signal was never missing** was this file's claim from 2026-08-04, and on 2026-09-23 it was
# **falsified and is corrected here rather than deleted**, because the correction is the lesson. CI
# does run on every push to `main` and has all along; what was wrong is the step after it, which is
# this script's own: that a green CI conclusion means a healthy tree. It does not. `ci.yml` skips
# the steps of the build jobs (`test`, `swish-check`, `boot-check`, and `swish-check-graphical` until its 2026-10-03 fold into `swish-check`)
# when every changed path matches `notes/`, `design/` or a root `*.md`, so a
# documentation-only commit posts a green required check having built and run nothing. That is a
# sound rule for a kernel and a false one for `crates/documentation`, whose tests RENDER those very
# files: `every_character_survives` never ran, `main` was broken for hours, and every signal a reader
# or this watcher could reach said green. Pull request #1168 fixes the skip. The durable lesson is
# about this file: **a conclusion is a claim about what ran, not about the tree.**
#
# The original point stands underneath it. On 2026-08-04 `main` went red and was found by someone
# running `script/lint` by hand after a merge, which is luck rather than process. This reads the
# signal that already exists, and now says what that signal does not cover.
#
# # What it says, and what it deliberately does not
#
# It reports the transition to red, naming the failing workflows, and the transition back to green.
# A green reached by citing a merge-group run rather than repeating it says so, with the run IDs.
# It does **not** report every red poll, because a trunk that stays broken for an hour is one fact
# and not twenty-four.
#
# It says "nobody is assigned to this" on purpose. A red trunk with an owner is a task; a red trunk
# without one is the failure this script exists to surface, and the wording is the difference.
#
# **Recovery is reported too, deliberately.** A watcher that only speaks on failure trains its reader
# to treat silence as health, and silence is also what a dead watcher produces.
#
# # Where this runs, and the watch that was folded in here and then unfolded (2026-09-24)
#
# **The trunk half runs in GitHub Actions as `nife-smelter[bot]`**, on a five-minute schedule:
# `.github/workflows/trunk-health.yml`, which carries the reasoning, the tested premise, and the
# cadence BUGS. It used to run under `launchd` on patagonia as calef's own token.
#
# **`helpers/at-risk-check.sh` did not come along, and could not have.** It was folded into this
# script's loop on 2026-09-23 on the reasoning that a third watcher is a third thing to start and a
# third thing that can die silently, and that `com.nife.trunk-health` was already firing on the
# right interval. That reasoning was correct for as long as both halves ran on the same machine.
# They no longer do: everything else here reads GitHub, while the at-risk check reads **this
# machine's** worktrees and their `git status`. A runner has no lane worktrees, so carrying the fold
# into Actions would have produced a check that reports nothing forever while the hazard it exists
# for sat on somebody's laptop unwatched, which is worse than not having it.
#
# So the at-risk watch is per developer, one per machine, under its own `launchd` job, and it needs
# no GitHub credential at all. notes/merge-queue.md has the plist and the retirement commands for
# the two jobs this replaces. `--once` here still reports it, because a person running this by hand
# on their own machine is exactly the case where both halves are true at once.
#
# # The thing that would prevent this rather than detect it
#
# GitHub's require-branches-to-be-up-to-date rule, applied 2026-08-04 (§73). Two pull requests, each
# green against the base it was cut from, merged in an order neither had ever been tested in and put
# `main` red. That rule forces a re-run against the new `main`, turning that failure into one re-run
# instead of a broken trunk. This script is the detection half; the rule is the prevention half.
#
# # BUGS
#
#   - **A green from a push that skipped its suites is sound, and says so** (A′, 2026-09-24). CI and
#     verify skip on a push to `main` when a `merge_group` run of the same workflow concluded
#     `success` at that exact commit, because the queue builds the very commit that lands. Those
#     runs conclude `success` with every gated job skipped, so this script reads them green, and
#     prints the merge-group runs it is relying on so a reader can tell that green from the one
#     below. It trusts the gate's lookup rather than re-deriving it: it does not check that the run
#     it names belongs to the same workflow as each skipped push run, only that some merge-group run
#     at this commit succeeded (CI or verify, whichever it found).
#   - **A green conclusion is not a healthy tree, and this script cannot tell the difference.** See
#     the correction above: a check whose steps were skipped reports `success`, so this watcher says
#     GREEN while the test that would have caught the breakage never ran. Nothing here reads whether
#     a job did any work, and a fix would mean reading each run's steps rather than its conclusion,
#     which is a different and much chattier API. Until then, a docs-only merge is a moment to
#     distrust this watcher rather than to be reassured by it.
#   - **It reports and never resolves**, deliberately, which was the whole gap calef named on
#     2026-09-23. The response now exists as `helpers/queue-hold.sh` and briefs/main-is-red.md, and
#     a person still has to run it: this script does not, because holding the queue needs the
#     judgement notes/merge-queue.md argues a watcher must not exercise.
#   - **It does not report its own death.** Shared with `helpers/merge-drain.sh`, accepted rather
#     than solved; notes/merge-queue.md has the reasoning and the `launchd` plists.
#   - **"Transition" means newly observed, not newly due.** The red/green, cadence and DUE memories
#     (`prev`, `prev_cadence`, `prev_due`) live in the watching shell and die with it, so a restart
#     announces the current state on its first pass as if it were new: a cadence that has stood due
#     for a month is re-reported once by every restart, and a due that clears while nothing is
#     watching is never reported at all. No persistent state sits behind any transition in this
#     script; that has been true of the RED/GREEN and cadence halves all along and is stated once,
#     here.
#
# Name: unrecorded. Provisional, minted 2026-08-04 and not yet put to calef. `trunk` rather than
# `main` because the branch could be renamed and the concept could not, and because "trunk health"
# is the term the field already uses. See notes/merge-queue.md.

set -e
cd "$(dirname "$0")/.."

REPO="nifeos/nife"
once=""
[ "$1" = "--once" ] && once=1

# A watcher must run from the MAIN checkout, never from a lane worktree, and this refuses rather
# than trusting anyone to remember. Measured cause, twice: `/bin/sh` reads a script LAZILY, so
# deleting the file under a running shell can kill it mid-loop. The merge drain died that way on
# 2026-08-18 when the worktree it was launched from was pruned, and `trunk-health.sh` died the same
# way later the same day during a 24-worktree cleanup, silently, while `main` was red on the
# fastpath gate for hours. The drain survived that second sweep only because it happened to have
# been relaunched with an absolute path into the main checkout.
#
# Only the watching form is refused. `--once` is a check anybody may run anywhere, including a lane
# gating its own work, and it exits long before a prune could reach it.
#
# `--git-dir` resolves to `.git/worktrees/<name>` in a linked worktree and to `.git` in the main
# checkout, which is the cheapest true test available; `--git-common-dir` points at the shared
# `.git` from both and cannot tell them apart.
if [ -z "$once" ] && [ "$(git rev-parse --git-dir 2>/dev/null)" != ".git" ]; then
	echo "$(basename "$0"): refusing to watch from a lane worktree." >&2
	echo "  A watcher outlives the lane that started it, and pruning that lane's worktree kills" >&2
	echo "  it silently, because /bin/sh reads a script lazily. Run it from the main checkout:" >&2
	echo "    cd <main checkout> && helpers/$(basename "$0") &" >&2
	echo "  ('--once' is fine from anywhere; only the watching form is refused.)" >&2
	exit 2
fi


state() {
	full=$(git ls-remote "$(git remote get-url origin 2>/dev/null || echo origin)" refs/heads/main 2>/dev/null | cut -f1)
	sha=$(printf '%.8s' "$full")
	[ -z "$sha" ] && { echo "unknown"; return; }
	runs=$(gh run list --repo "$REPO" --branch main --limit 12 \
		--json workflowName,status,conclusion,headSha 2>/dev/null || echo '[]')
	failed=$(printf '%s' "$runs" | jq -r --arg s "$sha" \
		'[.[] | select(.headSha[0:8] == $s) | select(.conclusion == "failure") | .workflowName] | unique | join(", ")' 2>/dev/null)
	running=$(printf '%s' "$runs" | jq -r --arg s "$sha" \
		'[.[] | select(.headSha[0:8] == $s) | select(.status != "completed")] | length' 2>/dev/null)
	if [ -n "$failed" ]; then
		echo "RED $sha $failed"
	elif [ "$running" = "0" ]; then
		# Say which kind of green (A′, 2026-09-24): a push the merge group already tested skips its
		# suites and concludes `success` having run nothing here, which is sound only because the
		# merge-group run at this same commit did run them. Naming that run is what separates it
		# from the skipped-docs green in BUGS below. An API failure prints the plain form.
		tested=$(gh api "repos/$REPO/actions/runs?event=merge_group&head_sha=$full&status=success" \
			--jq '[.workflow_runs[] | select(.name == "CI" or .name == "verify") | "\(.name) \(.id)"] | join(", ")' 2>/dev/null || true)
		if [ -n "$tested" ]; then
			echo "GREEN $sha (merge group tested this commit: $tested)"
		else
			echo "GREEN $sha"
		fi
	else
		echo "PENDING $sha"
	fi
}

# **A scheduled workflow's red is invisible to `state()` above, structurally rather than by
# oversight**, and that is why this second question is asked here (milestone 238). `state()` filters
# runs to `headSha == main`'s current tip, which is the right filter for "is the trunk broken" and
# the wrong one for a cadence: a weekly job's run matches the tip only until the next merge, and at
# this tree's merge rate that window is minutes. So `mutation testing` failed four Mondays running
# and this watcher, pointed straight at the same API, could not have seen any of them.
#
# `script/cadence-check` is where the judgment lives; this only decides when to speak. Same
# transition discipline as RED/GREEN, for the same reason: a cadence that has been dead for a month
# is one fact, not four hundred polls of it. `|| true` because a dead cadence is its exit 1 and this
# script runs under `set -e`.
#
# **Two of the cadences are asked here rather than by `script/cadence-check`** (milestone 889 (the
# tree's gates stop depending on the fleet tooling), provisional, 2026-10-12 UTC). The weekly capture
# of machine effort and the two merge watchers measure the fleet that runs agent lanes, and
# cadence-check is a tree tool that a stranger's clone keeps, so it no longer calls either. Both
# are asked on the same five-minute pass as before, in the same row format, so the transitions and
# trunk-health.yml's DEAD warning read them exactly as they did.
#
# The effort capture (milestone 519 (what this project costs, tracked where it cannot rot)) is a
# person running `script/effort --update` on the machine the lanes ran on, because only that
# machine has the session records; `--check` asks whether the committed `effort.csv` holds the
# current week, which needs no network and no records. It prints its row only when dead, the
# `--quiet` rule. The merge watchers (milestone 723 (a stopped merge watcher is reported within
# three of its own intervals)) have their own rule, from their own cron, in helpers/watcher_watch.py.
cadence() {
	script/cadence-check --quiet 2>/dev/null || true
	if [ -x script/effort ]; then
		effort_say="$(script/effort --check 2>&1)" ||
			printf '%-32s %-24s %s\n' "effort capture" "not this week" "DEAD: ${effort_say#effort: }"
	fi
	python3 helpers/watcher_watch.py check --quiet 2>/dev/null || true
}

# --- DUE: a cadence that says "a person must now do something" (milestone 495) ------------------
#
# **A due cadence is not a dead one, and the two cadences that mean "due" reached this watcher only
# as the second thing.** The stranger cadence (`script/stranger-test --due`) and the audit cadence
# (`script/audits --due`) both exist to say "a person must now do something", and red in the Actions
# tab is their correct output, not their failure. `script/cadence-check` answers only whether a
# scheduled workflow has stopped *succeeding*, so a job correctly reporting due and a job that is
# broken arrive as the same verdict, DEAD, a fortnight after the fact. The audits sat that way for
# five Mondays before anyone acted; that sentence with dates is milestone 495's evidence section.
#
# So this section asks the scripts themselves, not the workflows that relay them. Both `--due` modes
# read records already on disk (the stranger note's run headings, the audit index), need no network,
# no toolchain and no credential, and answer in seconds on the checkout this watcher already holds.
# That is milestone 495's option 1, taken for the reason it gives: it reuses the watcher a
# maintainer session is already told to confirm is alive, adds no scheduled workflow to the ones
# cadence-check polices, takes no `issues: write`, and opens no second channel. It is deliberately
# not a gate and not in `script/lint`, for the reason `script/cadence-check`'s header already wrote
# down: a due cadence is not a defect in the commit that happens to trip it.
#
# **The verdict speaks on transition, like everything else in this loop.** A cadence that stands due
# for a month is one fact, and the transition discipline is also what keeps DUE from becoming the
# standing alarm the block warns a due-echo would be: speak when something becomes due, speak when
# it clears, and not on the polls between. Clearing means the work was done (a run written up, or
# audits run and their rows added), or that the record moved underneath; both scripts' own BUGS
# sections name the edit-to-silence hole, and this inherits it rather than policing it. The recovery
# line, like RED/GREEN's, exists because a watcher that only speaks on failure trains its reader to
# treat silence as health.
#
# **A record that would not read gets a row rather than silence.** Exit 1 from a `--due` mode means
# "due" and also "the record could not be read" (a malformed or missing one), and the second case
# prints nothing on stdout, so the DUE sentence itself is what tells the two apart. Saying nothing
# about the second would recreate, in miniature, the silence this section exists to close.
due() {
	rows=""
	if st="$(script/stranger-test --due 2>/dev/null)"; then
		: # exit 0: a run is recorded as not due, and there is nothing to say
	else
		why="$(printf '%s\n' "$st" | sed -n 's/^stranger-test: DUE\. //p')"
		if [ -n "$why" ]; then
			last="$(printf '%s\n' "$st" | sed -n 's/^Last run: [0-9]*, \([0-9-]*\),.*/\1/p')"
			rows="$(printf '%-32s %-24s %s\n' "stranger-test" "last run ${last:-?}" "DUE: ${why}")"
		else
			rows="$(printf '%-32s %-24s %s\n' "stranger-test" "cannot ask" \
				"DUE: unknown; script/stranger-test --check says what is wrong with the run record")"
		fi
	fi
	if au="$(script/audits --due 2>/dev/null)"; then
		: # exit 0: no audit kind is overdue
	else
		kinds="$(printf '%s\n' "$au" | sed -n 's/^audits: [0-9]* on record, DUE: //p')"
		if [ -n "$kinds" ]; then
			n="$(printf '%s' "$kinds" | tr ',' ' ' | wc -w | tr -d ' ')"
			row="$(printf '%-32s %-24s %s\n' "audits" "$n overdue" "DUE: ${kinds}")"
		else
			row="$(printf '%-32s %-24s %s\n' "audits" "cannot ask" \
				"DUE: unknown; script/audits --check says what is wrong with the audit index")"
		fi
		if [ -n "$rows" ]; then
			rows="$rows
$row"
		else
			rows="$row"
		fi
	fi
	if [ -n "$rows" ]; then
		printf '%s\n' "$rows"
	fi
}

# Relayed by `--once` only; the watching form no longer calls it. See the header section above for
# why the fold was undone. `helpers/at-risk-check.sh` reports and never acts; this only relays it,
# and on a machine with no lane worktrees it prints nothing rather than being wrong.
at_risk() {
	helpers/at-risk-check.sh 2>/dev/null || true
}

if [ -n "$once" ]; then
	s=$(state)
	case "$s" in
	RED*) echo "main is RED: ${s#RED }" ;;
	GREEN*) echo "main is green at ${s#GREEN }" ;;
	*) echo "main: ${s}" ;;
	esac
	c=$(cadence)
	[ -n "$c" ] && printf '%s\n' "$c"
	d=$(due)
	[ -n "$d" ] && printf '%s\n' "$d"
	a=$(at_risk)
	[ -n "$a" ] && printf '%s\n' "$a"
	exit 0
fi

prev=""
prev_cadence="unknown"
prev_due="unknown"
while true; do
	s=$(state)
	case "$s" in
	RED*)
		[ "$s" != "$prev" ] && echo "MAIN IS RED at $(echo "$s" | cut -d' ' -f2) -- failing: $(echo "$s" | cut -d' ' -f3-) -- nobody is assigned to this"
		;;
	GREEN*)
		case "$prev" in
		RED*) echo "main recovered at $(echo "$s" | cut -d' ' -f2)" ;;
		esac
		;;
	esac
	prev="$s"

	# Recovery is announced here too, and for the reason the header already gives about RED/GREEN: a
	# watcher that only ever speaks on failure teaches its reader that silence means health, and
	# silence is also what a dead watcher sounds like.
	c=$(cadence)
	if [ "$c" != "$prev_cadence" ]; then
		if [ -n "$c" ]; then
			printf '%s\n' "$c"
		elif [ "$prev_cadence" != "unknown" ]; then
			echo "every scheduled workflow is producing results again"
		fi
		prev_cadence="$c"
	fi

	# The DUE half keeps its own transition memory instead of folding its rows into the cadence
	# stream above, because what clears a due list is that a person did the work, and "every
	# scheduled workflow is producing results again" would say the wrong thing about that. Same
	# shape and same first-pass rule as prev_cadence: a fresh watcher announces what stands, which
	# is the "newly observed, not newly due" caveat in the BUGS list.
	d=$(due)
	if [ "$d" != "$prev_due" ]; then
		if [ -n "$d" ]; then
			printf '%s\n' "$d"
		elif [ "$prev_due" != "unknown" ]; then
			echo "nothing is due any more"
		fi
		prev_due="$d"
	fi

	sleep 90
done
