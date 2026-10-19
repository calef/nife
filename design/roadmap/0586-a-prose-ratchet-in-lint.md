---
status: BUILT
built: 2026-10-19
raised: 2026-09-24
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 586. A prose ratchet in lint

Built. Minted by the maintainer on 2026-09-24, at the merge of the two decisions it
enforces: [§212 (a prose budget)](../decisions/0212-a-prose-budget-for-every-document.md) and
[§213 (writing standards)](../decisions/0213-writing-standards.md). calef ratified both on 2026-09-23.

Nothing blocks it. Both decisions are DECIDED and describe the check.

## Why one milestone serves both decisions

Each section asks for a gate, and the two gates are one mechanism with different counters. Both read
a document's main body with code fences stripped and blocks split before anything else is counted.
Both keep a per-file baseline that may not rise, the shape of the unsafe-count ratchet already in the
tree. Both let a new document, or one rewritten wholesale, meet the standard outright. And both
accept a marked exception carrying its reason. Building them as two milestones would build the
markdown body reader and the baseline file twice, or sequence one lane behind the other for no gain.

The decisions stay separate because they bound different things. The gate does not need to be.

## What it would be

A check in `script/lint` that measures every markdown document in the tree and compares it with a
committed baseline:

- Words of main body, against §212's 3,000-word cap. Appendices are measured under the same cap.
- Median and longest sentence, against §213's 20 and 40 words.
- Bold spans per 1,000 words, against §213's 4, reported as two counts: bold that opens a line and
  bold inline. They have different fixes.
- An orphan check: an appendix nothing links to fails, since §212's convention manufactures exactly
  that failure if nothing watches.

A document at or under every limit passes. A document over a limit passes only if it did not get
worse against its baseline, or carries a marked exception with a reason.

## The trap it inherits

Sentence splitting must respect block boundaries first. §213 records a naive pass that let a
paragraph's last sentence swallow the next heading. It reported a corpus median of 30 words where the
truth was 20. A gate built on that splitter fails documents that pass.

Markup a script parses, such as a roadmap block's `**Status:` line, counts as bold under §213. That
is a marked exception in §213's `BUGS`, not a carve-out the gate should invent. Decision files no
longer carry one: #1195 moved their status into YAML frontmatter on 2026-09-24, and the gate strips
frontmatter before it counts anything, so a decision's status is neither words nor bold.

## Design note, 2026-09-24: quoted text is exempt from the sentence limits

The sentence-length check must not count text inside quotation marks against the 40-word limit or
the median. A verbatim quote cannot be rewrapped. Splitting it changes what the speaker said, and
the tree's quotation convention (and `script/citations`, for attributed quotes) holds quotes word
for word.

The evidence is `notes/stranger-test/run-5.md`, condensed to §213 on 2026-09-24. Its only sentence
over 40 words is a stranger's worst-thing answer, quoted verbatim in `*"..."*`. Every other
sentence in that file and its seven sibling appendices was brought under the limit. A gate without
this exemption would fail that file for the one sentence it must not change.

The exemption covers the quoted span, not the sentence around it. A lead-in wrapped around a quote
is still prose and is still measured with the quote removed. Bold inside a quote still counts, since
bold is the writer's markup rather than the speaker's.

## Design note, 2026-09-24: an exception marker's count must match the file

A marked exception records the word count calef granted. Nothing compares that number with the
file afterwards, and the 2026-09-24 documentation audit found both marked exceptions past it the
same day they were granted. `design/fatal-risks/README.md` was granted 4,235 words and measured 4,440, after
a correction to risk 2 landed. `AGENTS.md`'s marker says 6,279 and the file measured 6,292. The gate
should treat the marker's number as the file's baseline: growth past it fails like growth past any
other baseline, and raising it is a new grant, which is an architect's.

calef ruled the same day to cut `design/fatal-risks/README.md` back rather than raise its grant, and it is
back at 4,235 words, within the grant, as of 2026-09-24. `AGENTS.md` was not part of that ruling.

## What was built, 2026-09-24

The check is `helpers/prose_ratchet.py`, run by `script/lint` as "the prose ratchet". Its header is
the manual. The baseline is `design/prose-baseline.tsv`, one row per document over a limit.

- Scope. The document list is `script/metrics`' prose-budget scope, moved into the module so the
  gate and the graph share it. Appendices were added to it. The graph's scope had missed every
  appendix, though its docstring said it counted them; that changes the series by one document.
- Measures. Words of main body, median and longest sentence, and bold as two counts. Fenced code,
  HTML comments, frontmatter and generated tables are stripped first. Blocks are split before
  sentences. A bold span may wrap onto the next line of its paragraph and still counts once.
- The ratchet. Over a limit passes only at or under the baseline row. A document with no row meets
  the limits outright. It is also compared with the merge base, so an unbanked shrink leaves no
  room to regrow.
- The baseline only shrinks. Rows may not be added or raised against the merge base, and a row for
  a missing file fails. A rename carries its row. `--bank` lowers rows and never adds.
- Exceptions. `<!-- prose-budget: exception. ... -->` is honored as `AGENTS.md` and
  `design/fatal-risks/README.md` already wrote it. `writing-standards` is its §213 twin.
  Each needs a date and a `Reason:`.
- The orphan check. A file under `X/` must be linked from `X.md` or `X/README.md`. That README is
  the directory's provenance page and is exempt. A thematic
  directory, `design/tenets/` today, is listed in the module and checked against its README. It
  found one orphan, `notes/project-metrics/ledger.md`, now linked.
- A selftest runs first. It holds each trap the reader must avoid, and each was confirmed to fail
  when its rule was broken.

### Four choices a reader should know about

Bold is held as counts, not density. Density is bold over words, so condensing a document raises it.
A density ratchet would fail the work §212 asks for. Each count may not rise; cutting words is free.
This departs from §213's words, "bold density may not rise", and calef may overrule it.
calef overruled it on 2026-09-26 (UTC): a document a change touches is judged on density, and
whoever condenses a document removes its bold too. §213 records the ruling in his words.

A mechanical rename is not a touch, either (calef, 2026-09-27T05:23Z, ruling on PR #1289): "a
mechanical rename that adds no new sentence does not count as touching a document for the
§212/§213 prose ratchet." A rename sweep's substitution ("calef's" to "an architect's", and forms
like it) adds a word almost everywhere it lands, which both checks above read as growth though
nobody wrote a new sentence. The exception is a mechanism: `RENAME_PAIRS` names the forms a
sweep produces, mapped back to the form each replaced. `rename_masked` reads a line through that
mapping before it is measured or diffed against the merge base, so a line whose only difference
from its old version is one of these substitutions reads as unchanged. A line that also carries a
new sentence still differs after the mapping, and still counts; the module's own selftest checks
this. A future rename of this shape needs a new pair in the list, not a new per-file exception.

Banking is lazy. A shrink does not force a baseline edit, because forced banking would put the
baseline in most pull requests. The merge-base comparison closes the slack instead.

The splitter differs from §213's in three ways, each a place the tree's prose broke it. A lowercase
word starts a sentence, since `calef` and the board names are lowercase. Inline code is one word,
since it is verbatim. Quoted text leaves before splitting, per the design note above. So its
corpus numbers run lower than §213's: the median document's median sentence is 17 words, not 20.

Median and density are not asked of documents under 200 words, §213's own floor. The longest
sentence is asked of every document.

### What it found on the day

The baseline holds 1,025 of 1,148 documents, regenerated from the tree it merged into. Longest
sentence is over in 905 and bold in 1,016. Words are over in 162 and the median in 178. Those counts are §212 and §213's debt, now held still.

New documents are the sharp edge. Of 209 added under this scope in the week to 2026-09-24, 163 would
have failed outright; 160 of those on bold, and only three because of parsed fields like `**Status:`.
So the gate will ask most new documents to change how they are written. That is what both decisions
ratified. When it was armed, 9 of 20 open pull requests would have failed it.

## What would make it not worth doing

If the baseline file churns on every pull request, the ratchet costs more attention than it saves.

## What the baseline churn costs, measured

`helpers/prose_baseline_churn.py` measures it from git alone, so any commit of `main` reproduces it.
The unit is a first-parent merge on `main`, one merged pull request. Read at 5635cb7da (2026-10-19):

| window | merged | changed a measured document | touched the baseline | forced |
|---|---|---|---|---|
| first 7 days, from the gate's merge at 2026-09-25T02:15Z | 176 | 149 | 6 (3.4%) | 1 |
| to 2026-10-19 | 1,050 | 877 | 45 (4.3%) | 4 |

What each number means:

- Touched counts a pull request whose diff changed `design/prose-baseline.tsv` at all. One of the
  first week's six is the merge that wrote the file.
- Forced is a touch the gate required. A renamed document carries its row, and a change of
  measurement re-measures the rows it moved. All four were tree-wide changes: two `measure:` bumps
  (#1328's roadmap frontmatter, #2070's whole-file word count) and two filename padding sweeps
  (#1822, #1823). No feature pull request was forced to edit the file.
- The other 40 touches removed or lowered rows by choice. 33 of them carry the `debt-paydown`
  label, lanes whose purpose was the backlog the file holds.
- No touch added a row, or raised one outside a change of measurement. The gate refuses both.

So the file did not churn. About one pull request in 25 touched it, and most of those were paying
down its debt. By this block's own test, the ratchet is worth keeping.

Where the comparison is not apples-to-apples:

- The file is not the gate's whole cost. A pull request rewritten to meet a limit before it merged
  leaves no trace in the baseline. That cost shows in failed lint runs, which git does not keep, and
  in exception markers. Six `writing-standards` markers stand in the tree and no `prose-budget` one
  (`helpers/exception_markers.py --list`).
- `--bank` lowers every row to the tree, not only the author's. #1483, a correction to one note,
  banked 677 rows that other pull requests had shrunk. On a busy file that risks merge conflicts,
  though none was recorded.
- The measured-document count reads today's scope at every merge. The scope widened at the gate's
  own merge, so early counts can include a document the gate of that day did not read.
- 149 of 176 merges changed a measured document because nearly every lane edits its own roadmap
  block. Touched over that count is no sharper a rate than touched over merged.

The baseline holds no rows as of 2026-10-19. With nothing to rename, lower or remove, only a change
of measurement can touch it, and the ratchet is a plain limit check on every document.

## The exception markers' names

`<!-- prose-budget: exception. -->` waives §212 and `<!-- writing-standards: exception. -->`
waives §213. Name: ratified 2026-10-19 (calef, PR #2425), each named for the decision it waives.
Refused `prose-ratchet: exception` (names the gate, not the rule waived) and `§212-exception` (a
number tells a reader nothing). `script/names` has no kind for a marker in an HTML comment, so
these two are recorded here and in `helpers/prose_ratchet.py`'s header, outside its register.

## Design note, 2026-09-29: a correction pays the same toll as a rewrite

The bold touch rule (calef's 2026-09-26 ruling) asks 4 bold spans per 1,000 words of any document
a change touches. Measured by the 2026-09-29 documentation audit: 864 documents stand over that
density, 634 of them in `notes/` and `design/roadmap/`, so a one-line correction to any of them
must first remove 25 to 82 bold spans. The audit met five such corrections and paid each with a
marked `writing-standards` exception (the line `notes/timed-wait.md` set on 2026-09-26), naming
the rewrite as the marker's debt. That includes `notes/documentation-audit.md`, the sweep
procedure, which is itself over the density and so cannot record a lesson without the same marker.
The bite on a document being condensed is the ruling's intent; the identical price on a correction
of a false claim is a consequence nobody measured until now. Whether corrections get a carve-out,
or the bold backlog gets a lane that retires these markers, is calef's call; this note records the
measured class so the next reader does not rediscover it one frozen document at a time.

## Design note, 2026-10-13: the word count is the whole file

The `words` measure is `budget_words`: every whitespace-separated word in the file, less a generated
table (MEASURE 3). calef ruled it on 2026-10-13 (UTC), choosing the option "Gate counts every
word"; §212 records it. §213's measures read prose, so its 200-word floor and bold density count
prose words. An exception marker's grant is read as `wc -w` without frontmatter (`counted_words`).

## Follow-on

- **Done.** Measured baseline churn over the first week and to date, in the section above. It
  finds the ratchet worth keeping.
- **Done.** The two exception markers' names, ratified by calef on 2026-10-19 (UTC). Where the splitter departs from §213's, and why, is in the module's header.
- **Done.** Built 2026-09-25: the marker-count check from the design note above. `granted_words()`
  reads the first number before `words` in a `prose-budget` marker as a whole-file `wc -w` ceiling.
  It waited on `AGENTS.md`, which #1285 brought to its 6,097. `design/fatal-risks/README.md` had grown to
  4,250 against its 4,235 through milestone 89 (Scaleway EM-RV1)'s table cell (#1278). Per calef's
  ruling of 2026-09-24 it was cut back rather than re-granted, and `main` then passed.
- **Done.** Resolved 2026-09-25: every new decision failed this gate, because the generated index
  table in `design/decisions/README.md` counted as prose (found when the queue removed #1278). The
  gate now skips a table under a heading in `GENERATED_TABLES`, the one `script/decisions` anchors
  on, as it skips fenced code (aae40a98f). `script/metrics` counts through the same set. The
  README's row went from 5,242 words to within the cap.
- **Refused.** A list of parsed markup exempt from the bold count. The maintainer asked for one on
  2026-09-24, citing the `Status:` and `Gate:` lines roadmap blocks and proposals must carry. It
  would overturn a ruling, so it is calef's call and was not built. §213 records calef's ruling of the same day:
  parsed markup is counted, no exclusion was carved, and frontmatter is the likely answer. Measured
  cost: of 209 documents added in the week to 2026-09-24, three fail bold only because of parsed
  fields. Proposals are outside the scope, so #1233's is not checked at all. If calef rules for the
  list, it is a set of line patterns subtracted in `bold_counts()`, about ten lines.

## Index row

Decisions 212 (a prose budget) and 213 (writing standards) were ratified with a ratchet as their enforcement and no gate built. This builds one check in `script/lint` for both: main-body word count, median and longest sentence, and line-opening and inline bold density, each held to a per-file baseline that may not rise, plus an orphan check for appendices. One milestone because both gates share the block-aware markdown reader and the baseline mechanism.
