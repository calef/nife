# §213's first measurement, and why it was wrong

Moved verbatim from [`0213-writing-standards.md`](../0213-writing-standards.md) on 2026-10-13 (UTC) to bring that
page under §212 (a prose budget)'s 3,000-word cap by the whole-file count. The page keeps the
heading, and the text below is unchanged.

## The first measurement was wrong, and the error is instructive

A first pass split sentences without respecting block boundaries. It read the median document's
median sentence as 30 words and its p90 as 64, and it reported 83% of documents over a 25-word median
and 97% over a 40-word maximum. Those four figures are inflated and the table above replaces them.

The cause: a heading, and a list item without a terminating period, both end without punctuation a
splitter recognises. So the last sentence of a paragraph swallows the heading after it, and one bullet
runs into the next. This section is its own example: read naively its longest sentence measures 73
words, and block-aware, 33. The bold and softener figures are unaffected, because neither depends on
where a sentence ends.
