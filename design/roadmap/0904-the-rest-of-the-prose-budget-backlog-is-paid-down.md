---
status: BUILT
raised: 2026-10-12
promoted_from: the-rest-of-the-prose-budget-backlog-is-paid-down
built: 2026-10-12
milestone_dependencies: none
decision_dependencies: 212
machine_requirements: none
specific_machine: none
needs_person: no
---
# 904. The rest of the prose budget's backlog is paid down

*(Promoted from the proposal pile on 2026-10-12 (UTC) by lane `maintainer/prose-cap-86-and-27`,
which brought its last two documents under the cap; number provisional until the merge queue lands
it. Title and slug are drafts.)*

Raised on 2026-10-12 (UTC) by the lane `maintainer/prose-budget-paydown`, which calef asked for that
day. That lane and its sibling `maintainer/prose-budget-decisions` (#1992 and #1994) split the
fifteen roadmap blocks and six decisions with the most words over the 3,000-word cap of
[§212 (a prose budget)](../decisions/0212-a-prose-budget-for-every-document.md). This file is the
work they found and did not do.

Reuse: no code is written. The tools are the tree's own: `helpers/prose_ratchet.py` measures and
banks, and the per-document method is the one #1992 and #1994 followed.

## What is left

On 2026-10-12 (UTC) the lane `maintainer/prose-budget-backlog` (#2005) brought 36 of the 38
documents this file listed under the cap. Each commit says what moved and why. They are:

- all 31 roadmap blocks;
- the note `notes/corrections/2026-09-23-the-sweep-that-swept-nothing.md`;
- §96 (process kernel or event kernel) and §101 (notification objects);
- §121 (what a device capability is when the device has no page);
- §123 (boot-time re-derivation).

Two decisions remained, 3,563 words over between them, and both waited
on calef.
[`0047-navigation-and-naming.md`](0047-navigation-and-naming.md) and
[`0139-drive-down-unsafe.md`](0139-drive-down-unsafe.md) are not in this list; each is a page
under the cap, done outside this milestone.

| decision | words over the cap |
|---|---|
| [§86 (whether an NVMe driver can leave the kernel)](../decisions/0086-el0-nvme-driver.md) | 3,300 |
| [§27 (the filesystem service)](../decisions/0027-filesystem-service.md) | 263 |

calef ruled on both on 2026-10-12 (UTC). For §86, "Split for §86.": no exception, and its long
sentences may be split as they move. For §27, "Also make §27 meet the 3000 word limit.", which
answered yes to the cut described below. The lane `maintainer/prose-cap-86-and-27` (#2015) did
both that day. §86's page is 2,964 words with two appendices, and §27's is 2,302 with one. Each
split only re-punctuates, adding "It", "it is" or "that is" where a split needs a subject. A
word-level diff of each old page against its new page and appendices shows no word removed.

One correction to the account below. For §27 it says the six-sentence cut moves the three
superseded accounts; it moves only the first. The rest of the amendment stays on the page, because
moving it as well would have split three more sentences than the six calef was asked about.

## How it is done

The method is the one #1992 and #1994 used. A block or note may be tightened as well as split, and
no fact, date or attribution is dropped. A decision is ruled text, so its paragraphs move whole and
nothing is reworded. Its pointer lines say only what each appendix holds, with no figures or claims
(the maintainer's send-back on #1994, under calef's 2026-10-11 ruling on #1919). Every new appendix
meets the §213 (writing standards) limits outright, so a decision paragraph holding a sentence over
40 words stays on the page.

## Why both were blocked

A decision's paragraphs move whole, and three limits then decide what can move. A paragraph holding
a sentence over 40 words breaks §213 (writing standards) in a new file. A new file of 200 words or
more must hold a median sentence of 20 and 4 bold spans per 1,000 words. And the page keeps every
bold span it does not move, so it must shed bold as it sheds words.

§86 cannot get under the cap by moving paragraphs. Sixteen of them, 2,483 words, hold sentences of 41
to 78 words, so none can go to an appendix. They include the prior-art list and the MSI-X
measurements. The smallest honest page is about 2,863 words, with a median sentence of 21 against
§213's 20. It waits on calef for one of two answers: a marked exception, or leave to split those
sentences as they move, as he allowed for §139 (who may read the cycle counter).

§27 has thirteen bold spans, so a page under 3,000 words must lose two of them. Only two paragraphs
carrying bold can move intact, and neither moves coherently on its own: one is the opening of the
2026-07-29 amendment, whose body cannot move with it. The superseded accounts, the natural thing to
move, hold sentences of 45 and 57 words. A search of every run of movable paragraphs, alone and in
pairs, found no cut that meets all three limits. The smallest change that works moves the
amendment's body (its account of the missing `TRUNCATE`, and the three superseded accounts) into an
appendix, with six long sentences split and two bold byte counts made plain. That rewords ruled text,
so it waits on calef the same way.

## Done when

Every document listed here is under the cap, or carries a marked exception with its reason, and its
row in `design/prose-baseline.tsv` has fallen to match.

## Follow-on

- **Done.** §86 and §27 came under the cap in PR #2015 (lane `maintainer/prose-cap-86-and-27`),
  and their rows in `design/prose-baseline.tsv` no longer carry a word count.
- **Done.** The two stale facts this block once listed under "Found on the way" were fixed by
  #2012 (lane `maintainer/fix-recorded-stale-facts`).

## Index row

The last two decisions over §212's 3,000-word cap, §86 and §27, split into a page and appendices
under calef's 2026-10-12 rulings, with each split sentence shown to keep every ruled word.
