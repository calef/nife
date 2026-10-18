"""Names calef has ruled and nobody has renamed yet, for `script/names`.

**Why this file exists.** A `Name:` block reading `provisional, and ruled: calef ruled **`x`** on
<date>` records a settled name whose rename has not been performed. `script/names --unratified`
listed it as plain provisional, so the maintainer re-asked calef three names he had already given.
This module reads the ruled form, so `--unratified` prints those names first under "ruled, rename
owed", and `--check` fails once one has waited longer than `MAX_AGE_DAYS`.

**The limit is 45 days.** Measured on 2026-10-17 (UTC), the oldest owed rename was 34 days old, so
a shorter limit would fail on day one and block every unrelated merge behind a backlog nobody can
clear at once. 45 passes today and fails a ruling nobody acted on within six weeks. An owed rename
that must wait (it is blocked on another milestone) is listed with an end date under "Rename-owed
exceptions" in `design/naming/convention-rulings.md`; the exception expires on its own date.

**Form only.** The parse takes the first backticked name after `calef ruled` and the first date
after it. `ruled the stem` rulings are not renames (the name already carries the stem), and a block
that says "and ruled" in a shape this cannot read is returned by `unparsed` so a reader sees it.

Name: provisional, minted 2026-10-17 (UTC) by the lane that built the ruled-rename-owed check. A
shared helper under `helpers/`, which `script/names` puts out of its own scope, so this paragraph is
the record. It sits beside `name_provenance.py` and `name_rules.py` and takes `name_` for the same
family; `name_renames` was considered and names the act, where the subject is the ruling. calef has
not named it.
"""

import datetime
import re
import sys

MAX_AGE_DAYS = 45
EXCEPTIONS_HEADING = "## Rename-owed exceptions"

_RULED = re.compile(r"^provisional,? and ruled\b")
_TARGET = re.compile(r"calef ruled (the stem )?\**`([A-Za-z0-9_-]+)`\**(.{0,200}?)\bon (\d{4}-\d{2}-\d{2})\b",
                     re.S)


def parse(text):
    """`(target, date)` for a block that says `provisional, and ruled:`, else None.

    None also covers a stem ruling and a ruling whose date or name is not in the usual spelling.
    """
    if not _RULED.match(text):
        return None
    m = _TARGET.search(text)
    if not m or m.group(1):
        return None
    return m.group(2), m.group(4)


def _readable(text):
    """True when the ruling has a target and a date, a stem ruling included."""
    return says_ruled(text) and _TARGET.search(text) is not None


def says_ruled(text):
    """True for a block that opens with the ruled form, readable or not."""
    return bool(_RULED.match(text))


def unparsed(records):
    """Provisional records that open with the ruled form and name no target and date at all."""
    return [r for r in records
            if r["status"] == "provisional" and says_ruled(r["text"]) and not _readable(r["text"])]


def owed(records):
    """Provisional records with a readable ruling, each given `ruled_to` and `ruled_on`."""
    out = []
    for r in records:
        got = parse(r["text"]) if r["status"] == "provisional" else None
        if got and got[0] != r["name"]:
            r["ruled_to"], r["ruled_on"] = got
            out.append(r)
    return out


def age_days(ruled_on, today):
    return (today - datetime.date.fromisoformat(ruled_on)).days


def exceptions(rules_text):
    """`{name: end date}` from bullets of the form ``- `name` until 2026-11-30: reason``."""
    out, inside = {}, False
    for line in rules_text.split("\n"):
        if line.startswith("## "):
            inside = line.strip() == EXCEPTIONS_HEADING
        m = inside and re.match(r"^- `([^`]+)` until (\d{4}-\d{2}-\d{2}):", line)
        if m:
            out[m.group(1)] = m.group(2)
    return out


def overdue(records, today, rules_text, limit=MAX_AGE_DAYS):
    """Problem lines for owed renames older than `limit` days and not excused by a live exception."""
    excused = exceptions(rules_text)
    out = []
    for r in owed(records):
        age = age_days(r["ruled_on"], today)
        end = excused.get(r["name"])
        if age <= limit or (end and today <= datetime.date.fromisoformat(end)):
            continue
        out.append(f"{r['kind']} {r['name']} ({r['path']}): calef ruled `{r['ruled_to']}` on "
                   f"{r['ruled_on']}, {age} days ago, and the rename is not performed (limit "
                   f"{limit} days, so a settled name is not left to be asked again). Perform the "
                   f"rename, or list it with an end date under \"Rename-owed exceptions\" in "
                   f"design/naming/convention-rulings.md if it is blocked")
    return out


def today_utc():
    return datetime.datetime.now(datetime.timezone.utc).date()


def _selftest():
    def rec(name, text, status="provisional"):
        return {"kind": "crate", "name": name, "path": f"crates/{name}/src/lib.rs",
                "status": status, "text": text}
    ruled = "provisional, and ruled: calef ruled **`good_name`** on 2026-09-13. The block stays."
    plain = rec("plain", "provisional, minted 2026-09-13 by a lane. calef ruled `x` on 2026-09-01")
    old = rec("old", ruled)
    new = rec("new", "provisional, and ruled: calef ruled `fresh_name` on 2026-10-10, to pair\n"
                     "with a program.")
    stem = rec("stem_protocol", "provisional, and ruled: calef ruled the stem **`stem`** on 2026-09-13")
    odd = rec("odd", "provisional, and ruled: the architect picked a name")
    recs = [plain, old, new, stem, odd]
    assert parse(ruled) == ("good_name", "2026-09-13"), "bold target"
    assert parse(new["text"]) == ("fresh_name", "2026-10-10"), "plain backticks, break between"
    assert parse(plain["text"]) is None and parse(stem["text"]) is None
    assert [r["name"] for r in owed(recs)] == ["old", "new"], "plain provisional is not owed"
    assert [r["name"] for r in unparsed(recs)] == ["odd"], unparsed(recs)
    today = datetime.date(2026, 10, 17)
    assert overdue(recs, today, "") == [], "34 days is under the limit"
    assert overdue(recs, datetime.date(2026, 10, 28), "") == [], "45 days is at the limit"
    late = overdue(recs, datetime.date(2026, 10, 29), "")
    assert len(late) == 1 and "old" in late[0] and "good_name" in late[0], late
    doc = "## Other\n- `old` until 2026-12-01: x\n" + EXCEPTIONS_HEADING + \
          "\n\n- `old` until 2026-11-30: waits on a milestone\n"
    assert exceptions(doc) == {"old": "2026-11-30"}, exceptions(doc)
    assert overdue([old], datetime.date(2026, 11, 30), doc) == [], "a live exception excuses"
    assert len(overdue([old], datetime.date(2026, 12, 1), doc)) == 1, "an exception expires"
    print("name_ruled: selftest ok")


if __name__ == "__main__":
    if sys.argv[1:] == ["--selftest"]:
        _selftest()
    else:
        sys.exit("usage: name_ruled.py --selftest")
