# Milestone 47 appendix: environment variables, designed and priced

An appendix to [milestone 47 (navigation and naming)](../0047-navigation-and-naming.md), moved out of
that page on 2026-10-13 (UTC) under [§212 (a prose budget)](../../decisions/0212-a-prose-budget-for-every-document.md).
The main page is complete without it. It holds the design of environment variables and the
2026-08-18 pricing that led to §111 (inert configuration is a read-only page, and each declared key
is validated against a closed domain). What was built is in [environment-built.md](environment-built.md).
Long sentences were split as they moved; no figure, date, name or quotation was dropped.

## Environment variables, which are the same question wearing a string costume

Clean slate: there is no `argv` and no `envp` today. `notes/abi.md` is explicit: "no libc, no
`argv`/`envp` array, no dynamic loader, no `main` wrapper", so a program gets argument words in
registers and a populated cspace. Nothing has to be undone. §15 (the native ABI: formalize the
convention, defer the BootInfo) already carries the natural seam as a deferred item: a BootInfo
page, "a structured block the loader hands the program".

Unix puts three different things in one string-to-string map. That is why environment variables are
both indispensable and a security disaster:

- Inert configuration (`LANG`, `TZ`, `TERM`). Genuinely just data, no authority in it.
- Names for finding things (`PATH`, `HOME`). This is namespace, and therefore *this milestone's*
  question: `HOME` is a directory capability wearing a string costume. `PATH` is "the set of
  directories I may spawn programs from", which is a set of capabilities.
- Secrets (`AWS_SECRET_KEY` and friends). These are authority badly encoded as a bearer
  string. In a capability system a credential is a capability to a service, not a value you can
  print, log, or leak into a crash dump.

So the three go three different places. Data stays data, names become capabilities (the work in
[absolute-paths-and-bind.md](absolute-paths-and-bind.md)), and secrets become endpoints.

The property worth designing for is not secrecy, it is that environment is an *open channel*. In
Unix anyone can set any variable and hope the program reads it. That makes every process carry an
unbounded implicit input. `LD_PRELOAD`, `IFS`, `PATH` and a long tail of library-specific variables
are attacks that work because a program can be influenced by something it never asked for and does
not know exists.

Invert it: a program declares the configuration it reads, and undeclared variables cannot reach
it. That is not a new mechanism. It is exactly what the SHILL-style manifest already does for
capabilities: a program declares its expected endowment, and the manifest is checked at spawn. A
mismatch is a refusal at the prompt rather than a mystery later. Configuration is the same shape,
and declaring it closes the entire `LD_PRELOAD` class by construction rather than by blocklist.

And no inheritance. Unix's environment is inherited by default. That is exactly why a secret in a
shell leaks into every child, including those with no business seeing it. Here it is granted like
everything else: at spawn, explicitly, visible in `caps`. The honest tension is the governing
constraint on [the main page](../0047-navigation-and-naming.md) (calef's objection to gratuitous
divergence). Environment variables are convenient *because* they are inherited, and full
explicitness is verbose. Proposed middle ground: inheritance with visibility. The shell holds a
default config set and passes it, but the passing is explicit and inspectable. So `caps run prog`
shows exactly what that program will see before it runs. Convenient in the common case, never
invisible.

One thing to decide deliberately rather than drift into. If configuration is declared in the
manifest, the manifest grows from "what capabilities do I need" into "what do I need at all". That
is a larger claim than it makes today. It is the sort of scope creep that is easier to accept early
than to reverse later.

### What it costs, measured 2026-08-18 rather than asserted (the absolute-paths lane)

This section argues the shape well and never prices the wire, which is the half that cannot be
undone. Four facts, each a lookup rather than an opinion, and the first changes the category:

- The spawn protocol is a userspace protocol, not the syscall surface. `spawnproto`'s own header
  says so: *"The kernel routes these words the way it routes any IPC; it never reads them. Adding a
  field is a change here, not to the syscall surface."* So an environment endowment is a change two
  programs agree on (the shell and init). That is still the irreversible category, and it is a rung
  below §10 (process model: capability-based, microkernel) and §16 (object revocation).
- The protocol already carries data rather than capabilities, and there is a precedent to copy.
  `DIR_BIT` announces "expect two more `SEND`s" and `GRANT_WORDS` carries three opaque words each.
  That is exactly the shape a bounded environment would take. Nothing new has to be invented to
  announce one.
- A page-shaped endowment already exists twice. One is the clock page a shell is granted read-only
  at a slot init names. The other is §15 (the native ABI)'s deferred BootInfo page, described there
  as "a structured block the loader hands the program". Init is the ELF loader and already maps
  pages into a child before starting it. So it is the one component that can place a table without
  a new mechanism.
- The receiving side is built and empty. `std::env` on nife is a process-local table
  (`sys/env/nife.rs`). `notes/std.md` already named this milestone's namespace as where a real
  endowment would come from to seed it, without changing the table's shape once it did. `temp_dir`
  already reads `TMPDIR` from it, so one variable steers a real behavior the day anything writes
  one. (Prediction borne out: see the "Built 2026-08-23" subsection in
  [environment-built.md](environment-built.md), and notes/env-config.md.)

Three encodings, with what each costs. They are not equivalent and the choice is an architect's.
The shell and init both read whatever is chosen, and a stranger's program is written against it.

| Encoding | What it costs | Where it fails |
|---|---|---|
| More `SEND`s on the spawn endpoint, the `GRANT_WORDS` shape | No page, no capability, no new VA. Three words a message, so ~24 bytes a pair and a fixed maximum count | A `PATH`-shaped value does not fit in 24 bytes, and raising the count means more round trips per spawn |
| A read-only page init maps (§15's BootInfo) | One frame per process, one fixed VA constant, and a parser crate both `user_rt` and the `std` PAL depend on (rule 7: two binaries agree on it, so it is a crate) | A page is 4 KiB and an environment is unbounded in principle; the page is a fixed cost even for the programs that read nothing |
| An endpoint to a configuration service | The most machinery by far: a server, a protocol, a slot | It is the right answer for the secrets third of this section and the wrong one for `TZ` |

Decided (calef, 2026-08-23, DECISIONS §111): the page, for the inert-configuration third only,
with the declaration in the manifest that closes the `LD_PRELOAD` class. The other two thirds stay
answered elsewhere in this milestone. Names become capabilities (the namespace work), and secrets
become endpoints (the broker shape of §41, the endpoint is the broker).

The page's layout, the irreversible part, is also settled. Each declared key is checked against a
closed, validated domain at assembly time rather than accepted as an arbitrary string: a real IANA
timezone identifier for `TZ`, a real locale code for `LANG`, a real terminal type for `TERM`. A
value that doesn't parse as a member of its key's domain is refused when the page is built, not
silently carried. This closes a real gap the recommendation above didn't originally address. A
value misclassified as "inert" (a secret typed into the wrong bucket by mistake) cannot ride through
disguised as configuration, because it cannot parse as one. `caps run prog`'s existing preview is
also extended to print inert-config *values*, not just key names. That catches anything a domain
hasn't been written for yet. See §111 for the full reasoning.
