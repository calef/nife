---
status: PARTIAL
raised: 2026-10-09
promoted_from: a-std-program-at-the-prompt-holds-the-network-and-a-resolver
milestone_dependencies: 384, 595
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 892. A std program at the prompt holds the network and a name resolver

**PARTIAL, 2026-10-12 (UTC).** Items 1 and 3 are built for `jig`, and a forget verb the block did
not foresee is built under them. Item 4 is met: its positive and refusal halves are proved in the
kernel harness, which calef accepted as the proof (ruling B on #2011, 2026-10-12 UTC). He approved
the forget verb as written (Q2, "Approve", the same day), and ruled item 3's fork, also the same
day ("yes, record c"). What is left is building item 3 for installed programs, under "Item 3,
ruled" below.

*(Promoted from the proposal pile on 2026-10-12 (UTC) by lane
`maintainer/promote-proposals`; number provisional until the merge queue lands it. Title
and slug are drafts.)*

Raised 2026-10-09 (UTC) by milestone 801 (packages over the internet)'s lane. It built a std
program resolving a name through `std::net` and fetching the package index over TLS, and could only
start it from the kernel test harness.

## The gap

Three pieces exist and are not joined at the prompt.

- The name resolver runs only when a test starts it. Milestone 384 (in a capability system the
  resolver is a grant)'s BUGS gave starting it from the lease to 801. 801 built the client half
  and left this, because nothing at the prompt could use it.
- A std program cannot hold the network at the prompt. Milestone 595 (the shell runs a `std`
  program)'s BUGS: the progenitor does not mint the socket frames' budget slot 3 needs, so
  `grant_plan` refuses a std manifest that declares the network.
- A std program's resolver is a badge at `std_runtime_protocol::RESOLVER_SLOT` (slot 9,
  provisional), placed today only by `system_tests`' `std_service`.

`jig` (milestone 809 (the package client becomes a program)) needs all three to reach
`basalt.nifeos.org` by name, so rung 3c's exit criterion on xenon waits on this.

## The work

1. The progenitor starts `name_resolver` once the lease arrives, told the lease's name server
   (`socket_protocol::lease`'s second word), as the test harness does with the runners' peer.
2. The progenitor mints a socket frames' budget for a std program whose manifest declares the
   network, and places it at slot 3. 595's `a_std_program_declares_only_what_the_std_layout_can_hold`
   then admits the network.
3. A manifest names the zone it may resolve; the progenitor grants a fresh badge that zone and
   places it at slot 9. Whether the zone is the manifest's word or the owner's is a fork: a manifest
   can only ask, and §252 (a resolver grant is one zone per client badge) says a spawner grants.
   For `jig` it is ruled: the root zone, recorded where it is granted (calef's Q5 on #1884,
   2026-10-10 (UTC)). For every other program it is ruled too: see "Item 3, ruled".
4. A `script/swish-check` line: an in-tree std program run at the prompt resolves a name in its zone
   and is refused one outside it, on all three architectures.

Reuse: everything named above is in the tree; this is wiring.

## Built 2026-10-12 (UTC), and a premise that was wrong

Lane `milestone/892-a-std-program-at-the-prompt-holds-the-network-and-a-resolver`. "Reuse: this
is wiring" was false. The resolver had eight windows for its life and no way to withdraw a grant
(`name_resolution_protocol`'s BUGS), and the progenitor outlives every job it spawns, so wiring
alone would have let the ninth `jig` of a boot be refused.

- The forget verb. `OPERATION_FORGET` (provisional) frees a badge's window; the resolver
  unmaps and deletes the page that badge attached, through its own address space at slot 4, as
  `net_stack` gives up a closed socket's page. Kernel test
  `a_forgotten_badge_is_denied_and_its_window_serves_the_next_client` fills every other window,
  forgets one, and requires the next badge to resolve there; without the `UNMAP` it answers
  `NOT_ATTACHED` (falsification patch in `system_tests/falsifications/`).
- Item 1. The progenitor starts `name_resolver` once the lease arrives, on either stack,
  told the lease's name server over UDP, and says so: `progenitor: name resolver up; asks
  10.0.2.3, the lease's name server`. `script/swish-check` requires that sentence on every boot
  with a network. One more resting capability slot: `CAPABILITY_TABLE_PEAK_MEASURED` is 34.
- Item 2 was built for `jig` by milestone 809 (the package client becomes a program).
- Item 3, for `jig`. `grant_plan::Prog::resolver_zone` (provisional) names the zone in the
  program table, not in `Manifest`, so installed bytes can never carry one. Each `jig` job whose
  verb holds the network gets a fresh badge granted the root zone (Q5), placed at slot 9, and
  forgotten at its reap. `caps jig install greeting@0.1.0` prints the row.
- Item 4. The swish script now starts nine `jig install`s in one boot. With the
  forget at reap removed, the ninth answered "could not spawn" on aarch64 (2026-10-12): the
  lifecycle is gated at the prompt. A name resolved at the prompt is not. Under slirp the lease's
  name server is always slirp's own proxy to the host's resolver, and QEMU refuses a `guestfwd`
  on its DNS address ("Conflicting/invalid host:port", measured 2026-10-12), so the runners' peer
  cannot answer there. calef ruled on #2011 (2026-10-12 UTC, "B") that the kernel harness's
  `a_std_program_resolves_its_granted_zone_and_nothing_without_a_grant`, green on all three
  architectures, is the proof that a program at the prompt resolves a name. No prompt line
  through the host's DNS and no name-server setting are owed.

## Item 3, ruled 2026-10-12 (UTC)

calef, on #2011: "yes, record c". For every program other than `jig`, the manifest asks for its
resolver zones. The owner grants them at `jig install`, in the same confirmation as the program's
other grants (§208 (installing a package is granting it, and the activation set is
versioned)). The owner can narrow them later. The details he accepted:

- A manifest may list several zones. Each granted zone is its own badge, per §252 (a resolver
  grant is one zone per client badge).
- A root-zone ask (any name, as a browser needs) is flagged prominently at install, not listed like
  the others.
- A program that asks for nothing gets no resolver.

What that leaves to build:

1. The zones in a program's manifest note, the ask. `manifest_note` is a format two programs agree
   on, so the encoding is decided once.
2. The granted zones in the activation set's entry, which is where an owner's narrowing lands too.
3. The progenitor grants one badge per granted zone at spawn and forgets each at the reap. The std
   PAL holds one resolver slot today, so a program with several zones needs a slot per badge, or
   a PAL that picks the badge whose zone holds the name.
4. The line at install: each zone listed, and a root-zone ask flagged apart. One confirmation
   covers it with the other grants. No milestone yet builds `jig install`'s listing of what it
   grants. The services milestone proposed on #2022 (a package declares a service) adds its
   "runs as a service" line to the same listing, and milestone 809 (the package client becomes a program)
   owns `jig install`. The listing belongs in one of them, and this item waits on it.

Not covered by the ruling: an installed `std` program cannot declare the network at all yet
(`grant_plan::image_can_carry` refuses it), so a zone is useless to one until that changes. Whether
the network is granted at the same confirmation is the next question for calef.

## BUGS

- Item 3 is built for `jig` only; "Item 3, ruled" says what is left.
- No program at the prompt resolves a name yet. `jig` holds the badge and fetches by a fixed
  address until milestone 809's item 6.
- A screen-narrowed job gets no resolver. The shell, not `job_undertaker`, reaps it, so the
  spawn service would never hear to forget its badge.
- The spawn service waits on the resolver. A grant or a forget is a `SEND` the resolver takes
  between requests, so a resolve nobody answers (three 15-second waits) delays the next spawn.
- At most eight resolving jobs at once; a ninth live one is refused rather than queued.

## Follow-on

- **Outstanding.** Item 3 for installed programs: the four pieces under "Item 3, ruled".
- **Outstanding.** Whether an installed `std` program may be granted the network at install, which
  item 3 needs first. calef's.
- **Milestone 809.** `jig` resolves the index's host through this badge at item 6.

## Index row

A `std` program started at the prompt can hold the network and a name resolver. Today the pieces
exist but join only when the kernel test harness starts them, so a package client cannot run from
the shell.
