---
status: REFUSED
raised: 2026-09-20
refused_by: 51, 448
---
# 484. IANA zone rules, and `strftime`

Refused by milestone 51 (design/roadmap/0051-wall-clock-time.md), which records it in its Follow-on.
Milestone 448 (design/roadmap/0448-a-refusal-gets-a-number.md) gives a refusal that names work a
number, a status and a condition that would change it.

## The refusal, in its own words

From '51. Wall-clock time, the `date` command, and an NTP service', under `## Follow-on`:

> The IANA tzdata is out and a fixed UTC offset is in, recorded in `notes/calendar.md`: zone rules
> are a data-distribution problem rather than a calendar one. There is no `strftime` either, five
> named formats instead.
>
> -- design/roadmap/0051-wall-clock-time.md

## Why it is here rather than only there

The calendar takes a fixed UTC offset and offers five named formats. Zone rules were refused on a
distinction worth keeping: they are a data-distribution problem rather than a calendar one. The
rules change several times a year, they arrive as a database, and a system with no way to ship and
update data cannot hold them honestly.

## Revisit

- **Condition.** A way to ship and update data on this system. That is the same precondition
  AGENTS.md puts on the ranking function itself, package management and an install story, so zone
  rules arrive when the thing that would carry them does. `strftime` is a separate and smaller
  question that the same bullet answered in passing.

## Index row

Time zones refused as a data problem rather than a calendar problem, which is the correct diagnosis
and which ties this refusal to a precondition the project has already named for other reasons. The
five named formats are what shipped instead.
