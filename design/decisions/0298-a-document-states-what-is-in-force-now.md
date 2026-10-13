---
status: DECIDED
raised: 2026-10-13
decided: 2026-10-13
ratified_by: calef
---

# 298. A document states what is in force now, and its history is in git

*Section number provisional until the merge queue lands it. The title and the slug are drafts, and
neither is calef's.*

## The ruling

calef, 2026-10-13 (UTC):

> We do not need the history in the text. That is what revision control is for. That's a rule so
> lets update our standards to ensure it.

A document in this tree says what is true and in force today. How it got that way is history, and
history goes in the commit message and the pull request body, where `git log` and `git blame` find
it. That covers replaced rulings, status timelines, "was X, now Y", "moved here from", dated
change narratives, correction stories, and which lane did what when.

It binds every Markdown document: `design/`, `notes/`, `briefs/`, the skills and `AGENTS.md`. §267
(a comment states the constraint as it is now) is the same rule for comments, and its amendment of
2026-10-09 is the precedent this one widens. §213 (writing standards) carries it as rule 5.

## What stays, and the form it takes

A reader needs some dates and some names to use the text as it stands. These stay:

- A ruling in force keeps its source and date once, beside the rule: "(calef, 2026-10-13 UTC)". A
  pull request number may follow as the citation. The ruling it replaced does not appear.
- calef's words stay where they are the rule, quoted. What he was reading, and who asked, do not.
- A measurement in force keeps its date and commit, because a reader judges and retakes it by
  them. An earlier figure goes, and so does the story of a measurement that was wrong.
- A refused option keeps its reason in the decision that refused it: "not X, because Y". That X
  was proposed, refused and proposed again does not.
- A `BUGS` entry states the limitation, its consequence and its tell. Who found it, and when, go.
- An open item states what is outstanding and what it waits on. Its status timeline goes.
- Status, and which section replaces which, live in frontmatter (`status`, `superseded_by`), where
  `script/decisions` and `script/roadmap` read them. No sentence in either section says one
  replaced the other. A replaced section stays in the tree, so every citation of its number still
  resolves.
- A provisional name is written `Name: provisional.`, then what was considered. Who minted it and
  when are in `git blame`.
- Text that moved leaves a pointer, a link that says where the text is. The page it went to links
  its parent. Neither says it moved.

When a rule changes, rewrite it in place. An amendment does not sit beside the words it replaced,
and `status: AMENDED` is how the frontmatter says the decision changed after it was first made.

## A record of an event is not history in the text

Some documents have an event as their subject, so their dates and their order are the content. A
correction of error under §210 (a correction of error), with its timeline, is one. So is an
audit report, and a dated record of one run or one measurement, such as a bench session or a
census. The gate below leaves them out by directory and by a date in the file's path. A page that
describes a mechanism, a rule or a milestone is never one of these, however much history it holds.

## The gate

`helpers/history_ratchet.py`, run by `script/lint`, is rung 2 of the `AGENTS.md` ladder for this
rule. It counts the phrases that history almost always uses, in Markdown outside code and
quotation. They are `supersede` in any form, `used to say`, `was ... until` a date or milestone,
`renamed from` and `moved from` a document. A heading such as `Timeline` counts too, and so does an
appendix named `history.md`. A branch may not add to the count, summed over the files it changes. A ceiling holds
`main`, and `--bank` lowers it. The helper's header lists every phrase and exclusion.

Measured 2026-10-13 (UTC) at `ff8941cee`, it counts 511 lines in 371 documents. In a sample of 60,
four were not history. Replayed over the 597 pull requests merged from 2026-09-29 to 2026-10-13, it
would have failed 62, two of them only because the replay could not follow a rename. Most of the
rest added a sentence this rule moves to git. A handful added none, such as a proposal naming the
decision it would replace, and those reword or carry an exception.

Existing text is a backlog, not a failure. Lanes pay it down per directory, and the gate only stops
it growing. A document whose subject is an event, outside the excluded paths, carries
`<!-- history: exception. Reason: ... -->`.

## BUGS

- The gate reads phrases. "The gate first held bold as two counts" is history and matches none of
  them, so review is still the mechanism, and the count is a floor.
- About one counted line in fifteen is not history: a regression described as "a property that used
  to hold", or a decision's table naming the rename it rules. Rewording it costs a sentence.
- Comments are not counted. §267's gate holds their length, not their phrasing, so a history clause
  in a short comment is left to review.
- The sample's classification is one agent's judgment.
