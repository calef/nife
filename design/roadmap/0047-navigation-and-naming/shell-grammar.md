# Milestone 47 appendix: the shell's grammar

An appendix to [milestone 47 (navigation and naming)](../0047-navigation-and-naming.md). The page is
complete without it. It holds why the shell has no `run` verb and no `file:` prefix, what a command
line means instead, and the mental model §141 (application is grant: what a command line means in a
capability system) records.

## No `run`, no `file:`

A bare program name spawns it (`worker 9`, `budgeter --mem 16`, `date`). A bare token in a file
position designates the file, and the manifest declares the direction. `--mem N` is accepted on
either side of the program name.

The parser does not classify tokens. `RunSpec` keeps the positionals in the order typed, and
`plan_against` places them into the slots the manifest declares. That is what makes "the manifest
says what it is" true in the code rather than only in the prose. A shape-based rule (a number is
the argument, anything else is the file) would read `wc 2026` as a missing file.

`caps <command>` is the preview. The tail is the command you would have typed, so what you inspect
and what you run cannot drift apart. It is the Unix prefix-word idiom (`time`, `nice`, `env`)
rather than new grammar. A program's own declaration is checked before what the shell holds. So
`worker report.txt` answers "takes no file; drop the name" (true whatever this shell holds) rather
than "you hold no such capability" (an accident of this boot). No shipped program declares
`FileSpec::Required`, so the "no such capability" refusal is reachable only through `plan_against`
in the host tests, not from the prompt.

`run` is refused on consistency, the DOS objection turned inward. Nobody would type `run ls`, so with
a verb, builtins are bare words while programs need a verb. A user would have to know *which class a
command is in* to know how to type it, which is the arbitrary divergence this milestone refuses.
Pipes make it worse: `run a | run b` is indefensible (milestone 50 (pipes and redirection)). The
lookup that replaces it is `Prog::from_name` and `dispatch`'s `Unknown` arm.

`file:` is refused because it announces the wrong half of the grant. `wc file:report.txt` reads and
`tee file:report.txt` writes: identical syntax, opposite authority. Direction lives in the manifest
by design; milestone 31 (a capability shell: designation is authorization) took the SHILL shape. So
the prefix marks the part already visible and stays silent on the part that matters. `worker 5
extra` is refused because worker's manifest says `FileSpec::Forbidden`, not because of any prefix.

The deeper reason: the capability claim is about absence, not presence. That a filename grants
access to that file surprises nobody. What `wc report.txt` proves is that wc got that file *and
nothing else*, and that claim lives in the tokens which are not on the line. A prefix decorating a
token that *is* present cannot express it. `caps <cmd>` can, including direction, which makes it the
visibility mechanism.

What survives: the manifest declaring direction; `caps` as the sole visibility surface; `--mem 16`,
a real grant with no Unix analogue spelled as an ordinary flag. Tests are the `crates/grant_plan`
host suite. Notes: grant-expression.md, program-manifest.md, date.md.

## Application is grant

The shell keeps whitespace application. Function-call notation (`wc(cat(f))`, `cat(f).wc()`) is
refused by §141, which measured each notation against the tree. What §141 keeps is the mental
model: `f(x)` means "spawn f, grant it x", so a nested command line *is* the authority tree, and the
delegation structure can be read straight off the syntax. No Unix shell can say that, because there
`f x` means "f can already reach everything, here is a string".

The pathology that call syntax was proposed to cure is word splitting: a value containing a space
re-split into two arguments after substitution, then `IFS`, `"$@"` versus `$@`, and glob expansion
firing at the wrong moment. The shell never re-splits a substituted value (notes/heap.md), which
cures it with no new syntax.
