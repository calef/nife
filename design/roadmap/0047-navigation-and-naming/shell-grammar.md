# Milestone 47 appendix: the shell's grammar

An appendix to [milestone 47 (navigation and naming)](../0047-navigation-and-naming.md), moved out of
that page on 2026-10-13 (UTC) under [§212 (a prose budget)](../../decisions/0212-a-prose-budget-for-every-document.md).
The main page is complete without it. It holds why `file:` and `run` came out of the shell, and the
function-call syntax fork that §141 (application is grant) later settled. Long sentences were split
as they moved; no figure, date, name or quotation was dropped.

## `file:` and `run` are not earned, and come out (decided 2026-07-30)

### Built 2026-07-31: the grammar change, ahead of the commands.

`run` and `file:` are gone from `grant_plan` and the shell. A bare program name spawns it (`worker 9`,
`budgeter --mem 16`, `date`). A bare token in a file position designates the file, and the manifest
still declares the direction. `--mem N` stays. It is now accepted on either side of the program
name, because with the verb gone a leading flag reads wrong.

The change the analysis below (which was written first) did not anticipate: the parser stopped
classifying tokens at all. `RunSpec` keeps the positionals in the order typed, and `plan_against`
places them into the slots the manifest declares. That is what makes "the manifest says what it
is" true in the code rather than only in the prose. A shape-based rule (a number is the argument,
anything else is the file) would have read `wc 2026` as a missing file.

`caps <command>` is the preview's new spelling. The tail is the command you would have typed, so
what you inspect and what you run cannot drift apart. It is the Unix prefix-word idiom (`time`,
`nice`, `env`) rather than new grammar. The refusals moved from "drop the `file:` designator" to
positional wording. One refusal order changed on purpose: a program's own declaration is checked
before what the shell holds. So `worker report.txt` answers "takes no file; drop the name" (true
whatever this shell holds) rather than "you hold no such capability" (an accident of this boot).
The consequence, recorded rather than glossed: no shipped program declares `FileSpec::Required`.
So the headline "no such capability" refusal is no longer reachable from the prompt, only through
`plan_against` in the host tests.

`date` came along with it. With `run` gone, `date` is exactly what a person types, and the shell had
never heard of it (`Prog` knew four programs). It has a `Prog` entry and an all-`Forbidden`
manifest. The shell spawns it with the register defaults, since `ArgSpec` has no position or arity
yet. It is the first program whose whole authority the command line cannot name: a read-only
mapping of the clock page, which init endows. This boot starts no clock service, so it prints "the
time is unknown: this process holds no clock capability", and `caps date` says so before you run
it. What a shell that could delegate a clock would need is assessed in notes/grant-expression.md
and is its own lane. It needs kernel boot wiring on both ISAs, a spawn-protocol position and both
inits, and nothing in the suite boots the interactive shell to prove any of it.

Tests: `crates/grant_plan` host suite, 34 cases. Notes: grant-expression.md, program-manifest.md, date.md.

calef asked to be convinced they were worth the typing. They are not, and the case against each is
stronger than the case that put them there.

`run` fails on consistency, the DOS objection turned inward. This milestone adds `ls`, `cd`,
`pwd`, `mkdir`, `rm`, and nobody would type `run ls`. So builtins become bare words while programs
need a verb. A user then has to know *which class a command is in* to know how to type it. That is
precisely the arbitrary divergence this milestone exists to refuse. Milestone 50 (pipes and
redirection) finishes it: `run a | run b` is indefensible. The lookup that replaces it already
exists (`Prog::from_name`, and `dispatch`'s `Unknown` arm). So `run` is phase-1 scaffolding from
when there were two programs, not a design position.

`file:` fails because it announces the wrong half of the grant. `wc file:report.txt` reads and
`tee file:report.txt` writes: identical syntax, opposite authority. Direction lives in the manifest
by design; milestone 31 (a capability shell: designation is authorization) took the SHILL shape
deliberately. So the prefix marks the part already visible and stays silent on the part that
matters. The safety argument fails too, on inspection. `worker 5 extra` is refused as unplaceable
because worker's manifest says `FileSpec::Forbidden`, not because of any prefix. The manifest was
doing all the work and the prefix was taking credit.

The reason it cannot carry the thesis is deeper than either. The capability claim is about absence,
not presence. That a filename grants access to that file surprises nobody. What `wc report.txt`
proves is that wc got that file *and nothing else*, and that claim lives in the tokens which are not
on the line. A prefix decorating a token that *is* present cannot express it. `caps run <cmd>` can,
including direction, which makes it the visibility mechanism and an argument for making it good.

What survives: the manifest declaring direction (load-bearing, untouched); `caps` as the sole
visibility surface; `--mem 16`, a real grant with no Unix analogue spelled as an ordinary flag.

Do it in this milestone, because the window closes. No program today takes both an argument and a
file (worker takes an int, budgeter takes memory, heeder and spinner take neither). So positional
resolution is at most one bare token and the manifest says what it is. Once a program wants both
(`grep pattern file.txt`), `ArgSpec` has to grow position and arity. The cost is that this changes
grammar in milestone 31 (a capability shell), which is built and host-tested. So the refusal wording
changes from "drop the `file:` designator" to something positional. Those tests are the work; it is
a contained edit, not a redesign.

## Open fork: should the shell be function calls rather than whitespace? (raised 2026-07-30)

Settled 2026-09-03 and refused, in DECISIONS §141 (application is grant). That decision kept this
section's one lasting idea as the system's mental model and measured each notation against the
tree. Kept below as history.

calef proposed `wc(cat(this-file.txt))` or `cat(this-file.txt).wc()`, on the grounds that shells lean
too hard on whitespace to tell a name from its arguments. Not decided. Recorded because the idea
contains one thing worth keeping whatever the syntax ends up being.

The diagnosis needs adjusting first. Whitespace is not ambiguous about which token is the
command; position handles that and always has. The real pathology is that a value containing a
space is silently re-split into two arguments after substitution. Then come `IFS`, `"$@"` versus
`$@`, and glob expansion firing at the wrong moment. Call syntax does cure it. But so does never
re-splitting a value, which costs no syntax and which we can adopt freely having no legacy.

The two proposed forms are not equivalent. `wc(cat(f))` is command substitution, not a pipeline:
the inner call must complete and return a value, which buffers the whole output. `cat(f).wc()` reads
in the direction data flows and is genuinely pipe-shaped, which is why `|>` exists in Elixir, F# and
OCaml. But a method implies an object with a type, and milestone 50 currently carries bytes. Over
bytes, `.wc()` is `| wc` with more punctuation, promising something the substrate lacks. Typed
pipelines are a separate and larger fork. They should be decided in milestone 50 on their own
merits, not smuggled in through notation.

The part that is genuinely ours: application is grant. `f(x)` means "spawn f, grant it x", so in
`wc(cat(f))` the nesting *is* the authority tree, and the delegation structure can be read straight
off the syntax. No other shell can say that, because in Unix both `f(x)` and `f x` mean "f can
already reach everything, here is a string". Worth writing down as the mental model regardless of
which surface wins, and it is a better answer than `file:` ever was.

Three objections. It costs more keystrokes than the `file:` this same milestone just deleted for
costing five. Bare `ls` becomes `ls()`, miserable interactively, so both spellings get allowed and
commands acquire two classes, which is the *same* objection that killed `run`. And shells are
optimized for typing where languages are optimized for reading. Oil/YSH, Elvish and Nushell all ran
at this, and Plan 9's `rc` is the one that worked, precisely by fixing quoting and word splitting
while keeping the terse surface.

The recommendation, if this is settled without further design: kill word splitting outright,
keep whitespace application with parentheses for grouping only (`wc (cat report.txt)`, the ML and
`rc` answer), and record "application is grant". That takes what the idea is pointing at and drops
the notation.
