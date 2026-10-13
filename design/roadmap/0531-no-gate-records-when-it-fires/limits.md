# Milestone 531 appendix: what to weaken, and what cannot be gated

The full case behind two sections of [milestone 531 (no gate in this tree records when it
fires)](../0531-no-gate-records-when-it-fires.md): the `script/cadence-check` false positives, and
the three things no gate can check. The main page is complete without this one, which exists to
verify or challenge it. "Today" means 2026-09-21, when this was measured.

## What to weaken, with evidence

`script/cadence-check`'s DEAD verdict is currently 67% false positive, measured today by running
it. Three workflows reported dead, of which one is true. `undefined-behavior check` has never had a
successful scheduled run, from 2026-08-10 to now, through two milestones that repaired it, and a
manual dispatch succeeding on 2026-09-17 is not a cadence. The other two are known-false by
construction. `audit-cadence`'s red is its signal, which the script's own header already records
as a case it cannot tell apart. Milestone 311 (the audit cadence tripwire, and the month of correct
alarms nobody acted on) is the record of what that costs. `vendor-watch` was monthly until
2026-09-17 and is now weekly, so its gap is a transition and not a death. That is the second half
of the false positive the header says will recur for the next non-weekly workflow anyone adds.

This is the tree's own "cries wolf" diagnosis applied to the watcher built to cure it, and both
fixes are already owed in the script's own header. Derive each workflow's expected interval from
the cron it already reads. And let a workflow declare that red is its signal, so the tripwire asks
"did it run" rather than "did it pass". Nothing else in this audit should be weakened.

## What cannot be gated

This is not a short section because the list is thin. It is short because the tree has already
written it: a grep for the declarative forms of "nothing checks this" returns 509 lines across
`notes/`, `design/` and `script/`, most of them deliberate, each sitting where a reader meets the
thing it is about. That density is the practice working, not a backlog.

The three that bound this proposal:

- A premise being overtaken. `design/fatal-risks/README.md` states it best about itself: a green
  `script/fatal-risks` means no status word contradicts the record it names, and is not a warrant
  that the arguments still hold. Milestone 275 (a gate that diffs `design/fatal-risks/README.md`
  against the roadmap it cites) closed the mechanical half and found four live disagreements on its
  first run. The larger half has no mechanism and this lane found no candidate for one. The
  alternative is the audit cadence, and the audit cadence is a tripwire whose red nobody acted on
  for five consecutive Mondays.
- Whether a comment, a gloss or a name is *true*. The tree's own worked example is exact. A
  `SAFETY:` comment describing capability validation sat above a `write_volatile` into a DMA page
  and passed `clippy::undocumented_unsafe_blocks` for as long as the file existed. That lint asks
  whether a comment is present and never whether it is about the thing underneath it. The
  alternative is the promotion triggers of §71 (a limitation is promoted when it stops being a fact
  and becomes a plan), and the audits, both of which are reading.
- Whether a green result means anything. `notes/check-inventory.md` answered that question by
  opening each file and asking, found six checks whose green is narrower than their name, and closed
  by saying there is no reason to believe six is the whole set. Addition B is the nearest a gate gets
  to it and it is not the same question: a selftest asks whether a check *can* fail, and this asks
  whether it fails on the thing its name promises. That gap is where the next finding will come from.
