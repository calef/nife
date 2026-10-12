#!/bin/sh
#
# Selftest: .githooks/pre-push runs no `gh` command on a checkout that has not opted in to the
# fleet tooling. Run by script/lint.
#
# Milestone 889 (the tree's gates stop depending on the fleet tooling), provisional. The hook used
# to ask GitHub two questions on every push from every clone; a stranger's push should pass or fail
# on the tree alone. This builds a throwaway repository with the real hook, stubs for script/fmt and
# script/lint (so it costs nothing), and a `gh` on PATH that records any call and fails. A push that
# changes a file goes through the whole hook, and the record must stay empty. Then the opt-in is
# set with no fleet helper in the tree, as in a clone with the fleet tooling moved out, and the hook
# must say the fleet checks did not run. helpers/pre-push-fleet-selftest.sh, run by
# script/lint-fleet, holds the fleet half, and proves the same stub sees a call when one is made.
#
# No network, and the real repository is never touched: every git command names the throwaway one.

set -eu
here="$(cd "$(dirname "$0")" && pwd)"
hook="$here/../.githooks/pre-push"
me="pre-push-selftest"

# A hook exports GIT_DIR, and this may run under one (helpers/scope-merge-base-selftest.sh has the
# 2026-10-03 incident). Clear it so every command below acts on the throwaway repository.
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_PREFIX GIT_COMMON_DIR GIT_OBJECT_DIRECTORY GIT_CONFIG_PARAMETERS

tmp="$(mktemp -d "${TMPDIR:-/tmp}/nife-pre-push-selftest.XXXXXX")"
trap 'rm -rf "$tmp"' EXIT HUP INT TERM
repo="$tmp/repo"
mkdir -p "$repo/script" "$repo/.githooks" "$tmp/bin"
g() { git -C "$repo" -c commit.gpgsign=false -c core.hooksPath=/dev/null "$@"; }

g init -q
g config user.name selftest
g config user.email selftest@invalid
cp "$hook" "$repo/.githooks/pre-push"
for s in fmt lint; do
	printf '#!/bin/sh\nexit 0\n' >"$repo/script/$s"
	chmod +x "$repo/script/$s"
done
echo one >"$repo/file"
g add -A
g commit -q -m one
before="$(g rev-parse HEAD)"
echo two >"$repo/file"
g commit -q -am two
after="$(g rev-parse HEAD)"

# The stub records its arguments and fails, as `gh` does offline.
calls="$tmp/gh-calls"
printf '#!/bin/sh\necho "gh $*" >>"%s"\nexit 1\n' "$calls" >"$tmp/bin/gh"
chmod +x "$tmp/bin/gh"

push() {
	: >"$calls"
	(cd "$repo" && printf 'refs/heads/main %s refs/heads/main %s\n' "$after" "$before" |
		PATH="$tmp/bin:$PATH" sh .githooks/pre-push origin "file://$tmp/remote")
}

if ! push >"$tmp/out" 2>&1; then
	echo "$me: the hook refused a push with every gate stubbed to pass:" >&2
	sed 's/^/  | /' "$tmp/out" >&2
	exit 1
fi
if [ -s "$calls" ]; then
	echo "$me: without nife.fleet the hook still called gh:" >&2
	sed 's/^/  | /' "$calls" >&2
	exit 1
fi

# The opt-in reaches the fleet half. Without its helper here the hook must say so rather than pass
# in silence, which is what a stranger's tree with the fleet tooling moved out would see.
g config nife.fleet true
push >"$tmp/out" 2>&1 || true
if ! grep -q 'nife.fleet is set but helpers/pre-push-fleet.sh is missing' "$tmp/out"; then
	echo "$me: with nife.fleet set and no fleet helper, the hook did not say the fleet checks were skipped:" >&2
	sed 's/^/  | /' "$tmp/out" >&2
	exit 1
fi

echo "pre-push selftest: no gh call without nife.fleet; the opt-in without its helper says so"
