# Milestone 47 appendix: environment variables as built

An appendix to [milestone 47 (navigation and naming)](../0047-navigation-and-naming.md), moved out of
that page on 2026-10-13 (UTC) under [§212 (a prose budget)](../../decisions/0212-a-prose-budget-for-every-document.md).
The main page is complete without it. It holds the record of the inert-configuration page as built
on 2026-08-23 and 2026-08-26, and why the secrets third was not built. The design and its pricing
are in [environment.md](environment.md). Long sentences were split as they moved; no figure, date,
name or quotation was dropped.

## Environment variables: what was built

### Built 2026-08-23, for a std program end to end. Not yet built: a shell customer. See notes/env-config.md.

The page was built, with everything it needed to reach a real process. `crates/env_proto` holds the
layout, the three closed domains, `PageBuilder` and `ConfigPage`, host-tested. A kernel-side
assembler and mapper is in `kernel/src/user/std_service.rs`, in the clock page's own shape: a
`Frame` with `READ` at a named slot, at a fixed VA the loader maps read-only. The `std` PAL's read
side is `patches/std-nife/overlay/std/src/sys/env/nife.rs::seed`, called once from
`pal::nife::init` before `main` runs. `std_exerciser`'s transcript proves the whole path on both
ISAs. The kernel assembled and validated `TZ=UTC`, `LANG=C`, `TERM=dumb` and mapped the page. The
program read those exact three values out of `std::env` without doing anything to ask for them.

What this section predicted correctly. All four "facts" the 2026-08-18 pricing found hold
without adjustment. The wire change is between two programs and not the syscall surface. No new
mechanism was needed beyond a page and a slot. `DIR_BIT`/`GRANT_WORDS`'s "announce, then read"
shape did not even need announcing, since the receiving program's manifest already says whether it
wants one, the same way `wants_clock` does today. And the receiving side really was "built and
empty": `sys/env/nife.rs`'s `ENV` table needed one new function, not a redesign.

What it did not predict. The manifest declaration called for by the "Invert it" paragraph (in
[environment.md](environment.md)) and by DECISIONS §111 (inert configuration is a read-only page)
does not exist yet. Nothing in `grant_plan::Prog` has a reason to read `TZ`/`LANG`/`TERM`. `date`
did not read the clock page until `date` existed to be its customer; the config page is in that
exact position. Concretely this means three absences. There is no `grant_plan::Manifest::config`
field and no wire bit on `spawnproto`. And the `caps run prog` preview extension §111 asks for has
nothing to preview. What *is* built is the kernel unconditionally granting a std program a config
page, the way it unconditionally grants one a clock. That is a demonstration harness standing in
for the shell's own "inheritance with visibility" default, not that mechanism itself. There is no
shell-held default config set yet, only `std_service.rs`'s one fixed default (`UTC`/`C`/`dumb`).

### The shell-facing customer was built 2026-08-26 (`milestone/47-remainder-round2`): `printenv`, `Manifest::config`, and real init wiring

This turned out to be exactly the wiring task the previous round's text predicted. There was no
fork underneath, unlike completion and `PATH` (in [path-and-completion.md](path-and-completion.md)).
Everything needed was `date`'s own shape, one field over, and every mechanism it reaches for
already existed.

- `grant_plan::Manifest::config: bool` (provisional field name) is `clock`'s twin, field for field.
  A program declares it, the shell cannot, and init reads the declaration to decide who gets the
  page. No wire bit on `spawnproto`, confirmed rather than merely predicted. `wants_config` is read
  straight off `prog.manifest().config` in `crates/system_initializer`'s spawn loop, the same line
  `wants_clock` already was.
- `Prog::Printenv` (provisional name) is `components/src/printenv.rs`. The name is Unix's own for
  exactly this, a term of art already right per this tree's naming convention for standard terms.
  It reads the page at a fixed VA (`CHILD_CONFIG_VA`, `crates/system_initializer`). It prints
  `KEY=value` for a declared key and `KEY (unset)` for one the page is valid but does not carry. It
  uses the same probe-before-touch shape `date` already uses for the clock: `granted(slot)` before
  building the pointer. So a process holding no capability at all never faults reading `CONFIG_VA`.
  `PROG_COUNT` moved 11 to 12, exercising `grant_plan`'s own round-trip sweep test as designed.
- Real init wiring, on both boards, not only the kernel test harness. `boot_config_page()`
  (`kernel/src/user.rs`) is `boot_clock_page`'s own twin minus the service. Nothing runs; the page
  is assembled once with the same `UTC`/`C`/`dumb` defaults. It is handed to init unconditionally,
  ahead of the filesystem pair, so its slot number is fixed on every boot whether or not a disk is
  attached. `BootEndowment::config_page` is a new field on both real entry points
  (`fixtures/src/hello.rs`, `user/src/system_initializer.rs`). That is why the filesystem pair's
  slot numbers moved by one on each board (documented at each call site). Nothing else's numbering
  changed, since config is granted unconditionally like the clock and cannot depend on whether a
  disk was attached. This is the wiring the 2026-08-23 lane named as missing. Its line "there is no
  shell-held default config set yet, only `std_service.rs`'s one fixed default" is no longer true
  for a std program's *test harness alone*. The real, both-board init path now grants the same
  default to any child whose manifest asks, `printenv` being the first and, for now, only asker.
- `caps printenv` prints a `config` row. It is `clock`'s row with the same one adaptation `date`'s
  page already required: presence only, not the three values. Printing values needs the shell to
  hold its own default config set to preview from. That is the "inheritance with visibility" middle
  ground named in the "Decided" passage of [environment.md](environment.md), which nothing has
  built yet; see "What remains" below.
- Proven with a real `printenv`, spawned directly with the grant `crates/system_initializer`'s
  wiring would make (`kernel/src/user/printenv_tests.rs`, both ISAs). The page's own three values
  round-trip unchanged from a real assembled page. They are deliberately not the boot defaults, so
  the test cannot pass by coincidence. A key the page never declared reads as `(unset)` rather than
  an empty string. A page nobody assembled (a zeroed frame) reads as no configuration at all rather
  than three empty strings. And a process granted no capability answers without touching
  `CONFIG_VA`.
- Also proven over the real interactive prompt. `script/swish-check`'s canned script now runs
  `printenv` and `caps printenv` against a real, both-board boot (`xtask::SWISH_CHECK_SCRIPT`).
  That is the same gate `date`'s own wiring is proven against. The reason is that
  `crates/system_initializer`'s per-command spawn logic (the `wants_clock`/`wants_config` match
  arms, the `caps`/`maps` assembly) is reachable only from a real init. It is excluded from the host
  pass for that reason, and `crates/system_initializer`'s own module doc says so. Nothing in
  `script/test`'s automated kernel suite goes through `crates/system_initializer::boot` at all;
  every lighter guest-test harness reimplements its own bespoke spawn loop instead.

Running that gate is what caught a real bug, and it is exactly the bug `script/swish-check`
exists to catch. The first swish-check attempt failed silently before any prompt. Init's own
capability table has sixteen slots. This file's own comment ("milestone 50 added two more kernel
grants... the shell's `build_child` had no slot left") already named that margin as having broken
once before; milestone 50 (pipes and redirection) is the one it names. A fourth permanently-held kernel grant (the
config page, beside the clock page and the filesystem pair) reproduced it. The fix moved
`for_test_roles`'s deletion from after console, line discipline and input are all built to before
any of them are. Nothing in between ever reads those capabilities' contents. The slots they freed
were sitting idle through the three builds that needed the room most
(`crates/system_initializer/src/lib.rs`; the comment at the new deletion site carries the account).
Verified by re-running `script/swish-check` on both boards after the fix. This is the argument for
the gate, made concrete. Nothing in the automated `script/test` suite would ever have caught this,
because nothing in it boots a real init.

What remains, honestly, is two things. One is printing the three inert-config *values* in a `caps`
preview, which DECISIONS §111 also asked for. It needs the shell to hold a default config set of its
own (unbuilt). The other is the "inheritance with visibility" mechanism itself, which is the same
unbuilt thing one level up. Neither has a forcing customer yet. `printenv` is read-only and
diagnostic rather than a program whose *behavior* would visibly change with a different default.

### Environment's secrets third: still not built, and the reason is sharper than "no PAL yet"

Read closely this round rather than re-asserted. The previous lane's line ("no PAL and no shell
support, unchanged") is accurate but understates why. Inert configuration had a simple build path
the moment it had a customer. Its *mechanism* (a validated, read-only page) was already fully
designed by DECISIONS §111 and by the 2026-08-18 pricing table in [environment.md](environment.md).
Nothing about secrets is in that position. "Secrets become endpoints (§41)" names the *shape* of the
answer, not a mechanism ready to extend. That is §41 (the endpoint is the broker).
`credential_protocol`'s `PROVISION`/`VERIFY` split is the one concrete instance of that shape in
this tree. It came from milestone 56 (secrets, credentials, and the entropy to make them safe). It
is purpose-built for *login*: an identity and a password-shaped secret, checked but never read back.

A generic secret, `AWS_SECRET_KEY`'s own case, is a different shape again. The *bytes* themselves are
often needed client-side (to sign a request), not merely a yes/no over them. So the credential
service's whole security property (a client can use a secret and never read it) does not transfer.
Generalizing `credential_protocol` into "any named secret, capability-gated" is itself an undecided
design question with real alternatives:

- a single broker service keyed by name and an ACL;
- per-secret minted endpoints an admin tool hands out;
- folding it into the login and session work of milestone 49 (users, login, and attribution), since
  that is what would actually *grant* a secret to a session in the first place.

And, per this milestone's own governing rule, there is still no forcing customer. Nothing in this
tree today needs a bearer secret the way a real network client eventually will. Building a generic
mechanism now would be inventing infrastructure against a hypothetical. This milestone has
correctly declined to do that for `PATH` and `bind` alike. Left exactly where it was, with the
reason on record rather than only the fact.
