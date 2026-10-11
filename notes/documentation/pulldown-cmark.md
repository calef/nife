# The roadmap said take `pulldown-cmark`. Two facts overrule it.

An appendix to [`notes/documentation.md`](../documentation.md), which states the conclusion.

It is not `no_std`. Version 0.13's `lib.rs` carries no `#![no_std]` and `parse.rs` uses
`std::collections::HashMap`. Taking it means either a permanent fork of somebody else's parser or a
std program.

And a std program on this system cannot be this program. The nife PAL's `Stdin::read` returns
`Ok(0)` unconditionally (`patches/std-nife/overlay/std/src/sys/stdio/nife.rs`) and there is no
argv. A std viewer could neither read a keypress to page nor be told which page to show. The
roadmap's premise that "milestone 27's std is what makes this buildable at all" is true of the
parser and false of the program.

What the roadmap could not weigh is the third fact: the corpus is closed and in-tree. A renderer for
this repository's own markdown does not need CommonMark conformance; it needs the constructs these
files use, and unlike conformance that is checkable directly. `every_character_survives` does
exactly that: every letter and digit of every note, decision and roadmap page reaches the rendered
output, in order. It found three real defects while it was being written (an escaped pipe read as a
column boundary, a table tail dropped when the buffer filled, a doubled blank line). That is three
more than a conformance suite over invented documents would have found here.

DECISIONS §46 rule 1 then settles it: this is on the verification path, so we write it. The crate is
`no_std`, zero-dependency, allocation-free on the guest path, and reachable by Kani.
