# The lint job's rename from `clippy`

Moved verbatim from [`../scripts.md`](../scripts.md)'s "CI leverages them" under §212 (a prose
budget) on 2026-10-13 (UTC). Name: provisional, minted the same day by lane/prose-cap-notes-c;
calef names files.

The lint job was named `clippy` until 2026-10-08 (UTC). It runs every check in `script/lint`, then
`script/lint-fleet`, and
a red `clippy` was usually a citation, decision or roadmap check. calef asked "Should the gate named
clippy be renamed? It seems to cause you some confusion when I cite it", then said "Go ahead with
lint". A required check is matched by name, so the rename goes in three steps, each safe
alone. First, `ci.yml` gains `lint` and keeps a `clippy` job that only re-reports `lint`'s verdict.
Second, once every open pull request has run that workflow, ruleset 19596094 requires `lint` in
place of `clippy`. Third, the `clippy` job is deleted. The second step, by the maintainer:

```sh
gh pr list --state open --json number,isDraft,statusCheckRollup --jq '.[] | select(.isDraft | not)
  | select([.statusCheckRollup[].name] | index("lint") | not) | .number'  # must print nothing
gh api repos/nifeos/nife/rulesets/19596094 --jq '{rules: [.rules[]
  | if .type == "required_status_checks" then .parameters.required_status_checks
    |= map(if .context == "clippy" then {context: "lint"} else . end) else . end]}' \
  | gh api -X PUT repos/nifeos/nife/rulesets/19596094 --input -
```
