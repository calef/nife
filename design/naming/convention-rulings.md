# Naming rules ratified for a class of names

*An appendix to [`design/naming.md`](../naming.md), which is the rule. This file holds the rulings
calef made for a whole class of names at once, with his words, and the list or the definition that
says which names each covers. `script/names` reads it (`helpers/name_rules.py`), so a covered name
counts as ratified and `script/names <name>` cites the rule. The directory `design/naming/` and this
file's stem are provisional names; naming is an architect's.*

A rule ratifies the names that follow it, today and later. A name that does not follow it is pulled
aside and keeps its own status, so a rule never ratifies a name nobody looked at by accident of
where it sits. Each section below says what it pulls aside and how that is decided.

## Rule 1: an appendix directory takes its parent's stem, and an appendix file a slug of its heading

Applies: appendix-stems
Ratified: 2026-10-17 (UTC)

The proposal, put to calef in the maintainer session of 2026-10-17 (UTC), and his reply:

> An appendix directory takes its parent's stem, and an appendix file takes a slug of the heading it
> was split from. Names that follow this are ratified by the rule. A name the parent's next rename
> changes follows automatically.

calef: "Ratified."

**What it covers.** A directory under `design/`, `notes/` or `briefs/` beside a document of its own
stem (`notes/stack/` beside `notes/stack.md`, the siting default of §212 (a prose budget)), and each
document inside it whose stem is made of the words of one heading of that parent. Measured on
2026-10-17: 103 such directories holding 507 documents, 164 of which follow the rule.

**What it pulls aside, leaving it provisional.**

- A stem that matches no heading of its parent. The check is mechanical (`_matches_heading`): every
  word of the stem, articles aside, must be a word of one heading. A stem with a word no heading
  holds was not derived from a heading. The check and the word rules together pull aside 343 of the 507.
- A stem that breaks a word rule a script can see (`_word_rule`): not lowercase and hyphenated,
  claims a Unix daemon (`daemon`, a trailing `-d`), or uses a British spelling `helpers/house_style.py`
  lists. The acronym test and the noun test need a reader and are not checked.
- A stem opening with a digit (a numbered or dated record), everything under `design/roadmap/`, and a
  directory named for an index (`notes/README/`).

Not covered: `design/fatal-risks`, `notes/coes`

Those two are records directories (calef, 2026-09-29 and #1921), which §212 separates from appendix
directories, so their names and the nine fatal-risks evidence appendices stay provisional.

**When the parent is renamed.** The directory follows its parent's stem and each file follows the
heading it came from, so the rename moves them together and no second ruling is needed.

## Rule 2: a test fixture program is named for its role in its test, as a noun phrase

Applies: fixture-roles
Ratified: 2026-10-17 (UTC)

The proposal and his reply:

> A test fixture program is named for the role it plays in its test, as a noun phrase. Names that
> follow this are ratified by the rule.

calef: "Yes"

**What it covers.** The 24 programs under `fixtures/src/` listed below. The list is explicit because
"names a role" is a judgement a pattern would guess at. A new fixture is covered by adding a bullet
here in the commit that adds it, and until then it is provisional like any other new name.

Covered:

- `carried_capability_client`
- `carried_capability_server`
- `confined_syscall_fuzzer`
- `console_test_client`
- `current_cpu_reader`
- `cycle_counter_reader`
- `delegation_granter`
- `delegation_receiver`
- `file_sink`
- `file_source`
- `frame_revoker`
- `image_self_checker`
- `memory_region_depleter`
- `name_resolver_test_client`
- `network_echo_client`
- `notification_binder`
- `page_frame_consumer`
- `page_frame_producer`
- `rendezvous_minter`
- `rendezvous_peer`
- `scratch_release_witness`
- `scratch_window_exerciser`
- `sink_transcript_writer`
- `unreachable_network_witness`

**What it pulls aside, leaving it provisional.** `alto`, `tenor`, `greeting`, `hello`, `noteless`,
`outlaw`, `chatty` and `flaky`. None names a role in its test: `alto` and `tenor` are two voices of a
duet, `greeting`, `hello` and `noteless` say what they print, and `outlaw`, `chatty` and `flaky` carry
rulings of their own that no rename has applied (`kernel_test_subject`, `swap_boundary_witness` and
`first_attempt_crasher`).
