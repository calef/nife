---
status: NOT-STARTED
promoted_from: a-quote-that-opens-a-sentence-is-counted-with-the-one-before
raised: 2026-10-12
milestone_dependencies: none
decision_dependencies: 213
machine_requirements: none
specific_machine: none
needs_person: no
---
# 925. A quote that opens a sentence is counted with the sentence before it

Raised on 2026-10-12 (UTC) by the lane `maintainer/prose-cap-86-and-27` (#2015), while splitting
§86 (whether an NVMe driver can leave the kernel) under §212 (a prose budget).

**In brief.** `sentences()` in `helpers/prose_ratchet.py` removes a quoted span before it splits on
sentence ends, and leaves `' \0'` in its place. The sentence-start lookahead in `SENTENCE_END` does
not accept `\0`, so a full stop followed by an opening quote is not a boundary. The two sentences
are measured as one. In §86's prior art, "Software mediation of NVMe queues exists, and it stops
at the admin queue." (13 words) and the 34-word sentence that opens with a quoted paper title were
counted as one sentence of 47 words. #2015 split the second sentence to pass, which the text did
not need.

Reuse: the fix is in the module's own splitter, one character class or one substitution. Nothing
outside the tree applies.

## What it would be

Let a quote's placeholder start a sentence when a sentence end precedes it, and add the case to
`selftest()`. A changed splitter changes measurements across the tree, so it lands with the module's
`MEASURE` bump and `--remeasure`, as that module's header describes. The baseline can only fall:
splitting one counted sentence into two never raises a longest sentence. A median can move either
way, which the remeasure records.

## Done when

`selftest()` holds a full stop followed by a quoted sentence as two sentences, and the baseline is
remeasured in the same change.

## Index row

`sentences()` in `helpers/prose_ratchet.py` fuses a sentence that opens with a quote to the one before it, so the prose cap measures two sentences as one. The fix is in the splitter, with a selftest case and a remeasured baseline.
