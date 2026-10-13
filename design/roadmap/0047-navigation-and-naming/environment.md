# Milestone 47 appendix: environment variables

An appendix to [milestone 47 (navigation and naming)](../0047-navigation-and-naming.md). The page is
complete without it. It holds how environment works on nife: where each kind of variable goes, the
rules that keep the environment from being an open channel, and the limits that stand today.

## Three things in one map, and where each goes

There is no `argv` and no `envp`. `notes/abi.md` is explicit: "no libc, no `argv`/`envp` array, no
dynamic loader, no `main` wrapper", so a program gets argument words in registers and a populated
cspace.

Unix puts three different things in one string-to-string map. That is why environment variables are
both indispensable and a security disaster. Here each goes to its own place:

- Inert configuration (`LANG`, `TZ`, `TERM`) is data with no authority in it. It stays data, on a
  read-only page, per §111 (inert configuration is a read-only page, and each declared key is
  validated against a closed domain).
- Names for finding things (`PATH`, `HOME`) are namespace. `HOME` is a directory capability wearing
  a string costume, and `PATH` is a set of capabilities. Names become capabilities: see
  [absolute-paths-and-bind.md](absolute-paths-and-bind.md) and
  [path-and-completion.md](path-and-completion.md).
- Secrets (`AWS_SECRET_KEY` and friends) are authority badly encoded as a bearer string. A credential
  here is a capability to a service, not a value you can print, log, or leak into a crash dump.
  Secrets become endpoints, per §41 (the endpoint is the broker).

## The environment is not an open channel

In Unix anyone can set any variable and hope the program reads it, so every process carries an
unbounded implicit input. `LD_PRELOAD`, `IFS`, `PATH` and a long tail of library-specific variables
are attacks that work because a program can be influenced by something it never asked for.

So a program declares the configuration it reads, and undeclared variables cannot reach it. It is
the SHILL-style manifest's shape: a program declares its expected endowment, and the manifest is
checked at spawn. `grant_plan::Manifest::config` is that declaration, `clock`'s twin. A program
declares it, the shell cannot, and the progenitor reads it to decide who gets the page. Declaring
it closes the entire `LD_PRELOAD` class by construction rather than by blocklist.

And no inheritance by default. Unix's environment is inherited, which is why a secret in a shell
leaks into every child. Here configuration is granted like everything else: at spawn, explicitly,
visible in `caps`. Full explicitness is verbose, and the main page's governing constraint is
against gratuitous divergence. The middle ground is inheritance with visibility: the shell's default
set reaches the child, and the passing is explicit and inspectable, so `caps printenv` shows exactly
what that program will see before it runs.

## The page

Each declared key is checked against a closed domain when the page is assembled: a real IANA
timezone identifier for `TZ`, a real locale code for `LANG`, a real terminal type for `TERM`. A value
that does not parse is refused when the page is built, not carried. So a secret typed into the wrong
bucket cannot ride through disguised as configuration. §111 holds the encoding choice and its
reasons.

The page is assembled once with the defaults `TZ=UTC`, `LANG=C`, `TERM=dumb` and granted
`READ`-only at a fixed slot and VA, the clock page's shape. The `std` PAL seeds `std::env` from it
before `main` runs. `printenv` prints `KEY=value` for a declared key and `KEY (unset)` for one the
page does not carry. It probes the slot before touching the page, so a process holding no
capability never faults. `caps printenv` prints the three values from the boot shell's own
read-only view of the same frame. notes/env-config.md has the files, constants and tests.

`crates/system_initializer`'s per-command spawn logic is reachable only from a real init, so it is
excluded from the host pass and from `script/test`'s kernel suite. `script/swish-check`, which boots
a real init on each board and runs `printenv` and `caps printenv`, is the gate for it. Init's
capability table has sixteen slots, and a permanently-held kernel grant can exhaust them before any
prompt appears; that gate is what catches it.

## Limits that stand

- A shell cannot choose a different set per child (`TZ=Europe/Paris printenv`). The page is the
  progenitor's, and one frame serves every child. Building it needs a per-spawn page and a change
  to the spawn protocol, and nothing needs it yet.
- A `login` session's shell cannot preview the values; only the boot prompt's shell holds the view.
- Both are in notes/env-config.md's BUGS.
- Secrets are not built here. The service that holds one is milestone 65 (a secrets service), and
  where a stored secret comes from is the open question of §165 (where a stored secret comes from on
  a boot that is not a test). A generic secret's bytes are often
  needed client-side (to sign a request), so the login credential service's property (use a secret,
  never read it) does not transfer to it.
