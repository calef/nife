---
status: PARTIAL
raised: 2026-09-05
milestone_dependencies: none
decision_dependencies: 154
machine_requirements: none
specific_machine: none
needs_person: no
---
# 388. An acronym sweep the tree can do at once

Filed 2026-09-05 as an unnumbered proposal by milestone 264, which asked
the acronym question of sixty names and deliberately answered none of them; numbered 2026-09-19 by
milestone 433 (drain)'s drain of the proposal pile. The rule behind the gate is
[§154 (acronym)](../decisions/0154-the-acronym-test-is-whether-the-phrase-is-spoken.md), which
replaced the 2026-09-05 test this block was proposed under.

an architect names things, and this is a list of names.

The sweep is performed. Every name this block carried is ruled and moved. The four in the
table below were ruled by calef on 2026-09-13 and performed by milestone 265 (proto), milestone
290 (one ntp binary is three programs) and milestone 298 (retire the multicast DNS responder).
The
five the 2026-09-05 rule deratified by name moved under §154 in the week that decision landed:
`asid` is `address_space_identifier` (#949), `gpt` is
`globally_unique_identifier_partition_table` (#955), `dtb` is `device_tree_blob` (#963), `ipc` is
`inter_process_communication` (#965), and the `dma` family followed on 2026-09-24 (#1229), when
calef ruled that the crate and its eleven kernel and driver identifiers expand in one change.

Each renamed crate's provenance block carries its ruling, its refusals, and a census that
classifies every survivor line. The classes that keep an acronym are recorded there, not here:
the blob rather than the crate (`let dtb`, `crate::DTB`), UEFI's own vocabulary (`GPT`, the GPT
header), the hardware field's manual spelling (`satp.ASID`, `asid_bits()`), IPC the concept
(`ipc_send`, the bench files), `dma` alone, and ACPI's `dmar`. This lane re-ran the token census
on 2026-10-15 and found no live identifier outside those classes. Two `ipc` module names minted
provisional by the 2026-10-12 file splits sit inside the IPC-concept clause and are on
`script/names`'s worklist, which is where a provisional name belongs.

What is left is one ruling, per name, on the two public kernel functions the renames deliberately
did not touch. `kernel/src/user/entropy_service.rs` still exports `jh7110_trng_device` and
`jh7110_crg_window`, so the file reads `jh7110_entropy::discover` inside a function called
`jh7110_trng_device`, and `jh7110_clock_and_reset::Found` inside one called `jh7110_crg_window`.
They are a genuinely harder case than the crates were, and that is why the 2026-09-13 renames
left them: unlike a crate name, each of these names the **hardware block**, whose device-tree
spelling is the vendor's (`starfive,jh7110-trng`, and mainline's driver file is `jh7110-trng.c`)
and whose boot-log wording throughout the tree is "JH7110 TRNG", so the "abbreviation we receive
rather than author" clause in design/naming.md may cover them where it did not cover the crate.
A lane should not guess: AGENTS.md puts public function names in an architect's hands. Both
functions now carry `Name: recorded` blocks citing this milestone, so they wait on
`script/names`'s worklist instead of invisibly, and the ruling is asked on the lane's pull
request with `needs-architect`.

## Why it cannot be done a name at a time

**An acronym is spelled out unless its expansion teaches nothing** (calef, 2026-09-05). The rule
deratified `dma`, `dtb`, `gpt`, `ipc` and `asid` in the sentence that set it, and design/naming.md
already says the sweep is its own milestone because `ipc` is load-bearing across the tree.

Milestone 264 found the second reason, which is smaller and sharper. **Several of these names exist
in matched pairs that a partial sweep would break.** The program `ntp` and the crate `ntp_proto` were
one word twice, and calef ratified the crate on 2026-08-23. Spelling out the program alone leaves the
pair disagreeing; spelling out the crate alone overturns a ratification as a side effect of tidying a
program. The same holds for `jh7110_trng`, which is a crate and the program built from it, a pairing
AGENTS.md describes as deliberate and worth seeing.

**That prediction was tested and held.** The `jh7110_trng` pair was ruled and performed as one
object both times, crate and program together, and the second ruling turned on the pair: bare
`jh7110_entropy_driver` was refused precisely because the crate is not a driver, which is a refusal
only visible if you are holding both halves at once. See `crates/jh7110_entropy/src/lib.rs`'s
provenance block.

## The names 264 surfaced, each with the question already asked

Every one of these carries the question in its own provenance block, recorded as open rather than
guessed at.

| Name | Expansion | The tension |
|---|---|---|
| ~~`ntp`, `ntp_proto`~~ | network time protocol | Expands into something more informative than itself, which is the deratified class. Against: it is the protocol's registered name and `ntp_proto` was ratified 2026-08-23. **Answered 2026-09-13: the stem is `network_time`**, deratifying that ruling; the crate is `network_time_protocol` since milestone 265 and the program was split away by milestone 290. |
| ~~`mdns_proto`, `mdns_config`, `mdns_responder`~~ | multicast DNS | The expansion contains a second acronym. A full spelling runs to `multicast_domain_name_system_proto` and has stopped teaching before it ends. **Answered 2026-09-13: `multicast_dns`**, DNS staying whole as the `pci` case one level down. Performed by milestone 265: `multicast_dns_protocol`, `multicast_dns_config`, `multicast_dns_responder`. |
| ~~`jh7110_trng`~~ (crate and program) | true random number generator | Expansion teaches, and the acronym is not one a reader outside hardware carries. **Answered 2026-09-13: `jh7110_entropy`**, via `jh7110_entropy_source` the same day. |
| ~~`jh7110_crg`~~ | clock and reset generator | Expansion teaches. Against: both device trees for this chip spell the blocks `syscrg`, `stgcrg` and `aoncrg`, so the acronym is the hardware documentation's own. **Answered 2026-09-13: `jh7110_clock_and_reset`**, the against-case overruled by the same-day amendment to decision 113, which ends the external-standard exemption for acronym crates. |

`cpu` and `icount` were asked and answered inside 264: `cpu` expands to something a reader already
has, and `icount` is a contraction rather than an acronym and is the exact string QEMU prints and
accepts.

## What settling it needs

Settled for the four names in the table, including the nested case (`mdns`), where the rule's own
asymmetry argument ran out: spelling out the outer acronym exposes an inner one, so the expansion
does not reach a word the newcomer knows either way. calef drew a line rather than applying the
rule harder, and the reasoning is in milestone 265's block and in
`crates/multicast_dns_protocol/src/lib.rs`'s provenance (that crate and its two siblings were
retired on 2026-09-15 by milestone 298; the provenance is at commit `0652c981`).

Settled for the five deratified names, each by a maintainer lane whose pull request and whose
crate's provenance block carry the ruling and the survivor census.

What is left is the BUGS below: one ruling, asked and waiting.

## Follow-on

- **Outstanding.** The ruling on `jh7110_trng_device` and `jh7110_crg_window`, asked on this lane's
  pull request with `needs-architect`. Checked 2026-10-15: both functions carry `Name: recorded`
  blocks citing this milestone and wait on `script/names --unratified`, and the token census for
  the five moved names found no live identifier outside the survivor classes their provenance
  blocks record.

## BUGS

- **`jh7110_trng_device` and `jh7110_crg_window` wait on calef's ruling, per name.** The question
  each answers: does the abbreviation the vendor gives the hardware block stay in our identifier
  for it, or does §154's test expand it the way it expanded the crates? The ask, with the census
  and the options, is on the lane's pull request. On a keep ruling, the `recorded` blocks are
  ratified in place and this block closes. On a rename ruling, the functions move and so do the
  identifiers that name the same blocks (`TRNG_VA`, `TRNG_BRING_UP`), while the vendor's
  `compatible` strings, the fixture files and the cited driver file stay received spellings.

## Index row

An acronym is spelled out unless its expansion teaches nothing (calef, 2026-09-05), and the rule
deratified five names in the sentence that set it. It cannot be applied a name at a time for two
reasons: `ipc` is load-bearing across the tree, and several of these names exist in matched pairs a
partial sweep would break, since spelling out a program alone leaves it disagreeing with its crate
and spelling out the crate alone overturns a ratification as a side effect of tidying a program.
That prediction was tested and held, on the `jh7110_trng` pair, where bare `jh7110_entropy_driver`
was refused precisely because the crate is not a driver, a refusal visible only while holding both
halves. The four names milestone 264 surfaced were all ruled by calef on 2026-09-13 and performed by
milestones 265, 290 and 298, including the nested case, where he drew a line rather than applying
the rule harder: DNS stays whole because it is the `pci` case one level down. The five deratified
names moved under §154 the week it was decided (#949, #955, #963, #965, #1229), each crate's
provenance block carrying the ruling and the survivor census. What is left is the larger half's last
two names: the public kernel functions `jh7110_trng_device` and `jh7110_crg_window`, which survived
the 2026-09-13 crate renames and are a genuinely harder case, because each names a hardware block
whose device-tree spelling is the vendor's.
