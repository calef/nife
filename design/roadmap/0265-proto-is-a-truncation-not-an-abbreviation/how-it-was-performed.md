# Milestone 265: how the rename was performed, and what the sweep took

Moved on 2026-10-12 (UTC) from
[`design/roadmap/0265-proto-is-a-truncation-not-an-abbreviation.md`](../0265-proto-is-a-truncation-not-an-abbreviation.md)
under §212 (a prose budget). The page holds the measurement, the rulings, the reasons, the
sequencing and the `BUGS`; this holds how the rename was carried out. Three bold spans became plain
text and two long sentences were split in moving.

The pattern was `git grep -lE '\b(<seventeen alternatives>)\b'`, word-anchored, never a substring
match, and the file list was written out and read before anything was edited. The two habits
`design/naming.md` prescribes both earned their keep:

- *Enumerate before sweeping* caught the false positives. Thirty-four distinct `*_proto` tokens
  exist in the tree and only fifteen are live crates. The rest are predecessors kept on purpose
  (`fs_proto`, `cred_proto`, `gfx_proto`, `sink_proto`, `env_proto`), names of crates that do not
  exist yet (`smb_proto`, `nfs_proto`, `manifest_proto`), and `ntp-proto`, which is ntpd-rs's and
  belongs to somebody else. A substring sweep would have eaten all of them.
- *A program's name is not compiler-checked* caught the false negatives for `mdns_responder`. The
  string sites were enumerated rather than trusted to `cargo check`. They are the `[[bin]]` name and
  path, the archive tuples in `xtask` once per architecture, the kernel's `program(...)` lookups and test
  expectations, `crates/timetable`'s fixture strings, the shipped `components/*.conf` compiled in
  with `include_str!`, and one derived identifier, `mdns_responder_image`.

The sweep took five records it should not have, and enumerating the diff is what found them. All
five are restored, each with a sentence beside it saying what the thing is called now:

- A refusal. `timebase_proto`'s *"Refused `timebase_proto`, above"* became a name refusing
  itself, which is this tree's oldest naming scar exactly.
- Two records of what the 2026-08-23 renames produced. `graphics_proto`'s block and
  `user_mode_runtime`'s both cite `cred_proto` -> `credential_proto`; the sweep made that day produce
  a name that did not exist for another three weeks.
- The 2026-07-30 four-ways account, carried identically by `clock_proto`, `entropy_proto`,
  `supervision_proto` and `swap_proto`, where the whole point of the sentence is that two of the four
  spellings ended in `_proto`. Rewriting them left it listing four spellings, two of which no longer
  contain the word being contrasted.
- `socket_proto`'s 2026-08-01 ratification, which named `socket_proto`.

And one gate went blind rather than red. `script/lint` check 3 globs `crates/*proto` to select
the crates whose suffix it then judges. After the rename that glob matches nothing, so the loop body
never runs and the check passes by checking zero crates, which is the `--exclude`-goes-stale failure
`design/naming.md` lists one row over. It now globs `crates/*protocol`, and the hazard is written into
its comment because the next suffix change will meet it again.

The status rule decided the markdown, 108 files swept and 64 left alone: `BUILT`, `REMOVED` and
`RECORDED` roadmap blocks, `design/decisions/`, `design/audit-reports/` and the `BUILT` rows of
`design/roadmap/README.md` are accounts and keep the spelling they were written in. Live intent
moved: `PROPOSED`, `NOT-STARTED` and `PARTIAL` blocks, every file under
`design/roadmap/proposals/`, and `notes/`.
