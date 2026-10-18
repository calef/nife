---
status: PROPOSED
raised: 2026-10-17
---

# 307. How long a name may be: one bound for programs, packages and the files they travel in

*(Section number provisional until the merge queue lands it.)*

Raised by calef, 2026-10-17 (UTC), while reviewing unratified names: *"32 bytes is probably too
short for packages anyways."* and *"I would like to get to 255 bytes for the filename like
Debian/Ubuntu."* The trigger was `usb_keyboard_driver`, which a flat rule spelling out every acronym
would make `universal_serial_bus_keyboard_driver`, 36 bytes, past both 32-byte ceilings below.

This changes formats that three programs share (nife's kernel and tools, `jig`, and basalt's
published packages), so it is decided before any code. Nothing here is built.

## What holds today

Two on-disk formats carry a name in a fixed 32-byte NUL-padded field, and many smaller ceilings sit
beside them. Every one was found by search on 2026-10-17 (UTC):

| where | bound | what it bounds | kind |
|---|---|---|---|
| `crates/nifefs` `NAME_LEN`, magic `CRKR0002` | 32 | an archive entry, so every program the kernel or the progenitor spawns by name | on-disk, fixed; `ENTRY_LEN = 40`, `MAX_FILES = 178` in 14 directory blocks |
| `crates/package_archive` `NAME_LEN`, magic `NIFEPKG1` | 32 | package name, version, architecture, and every member | on-disk, fixed; header 112 bytes, member entry 72 |
| `package_archive::PROGRAM_PREFIX` | 28 | a packaged program, since its member is `bin/<program>` in a 32-byte field | derived |
| `package_archive::STEM_LEN`, `swish` `bare.rs` `PATH_MAX` (108), its `Label` (32) | 98, 108, 32 | the `name-version-arch` stem and `/packages/<n>/<v>/<a>` | runtime buffers |
| `filesystem_protocol::grant::MAX_NAME`, `grant_plan` `MAX_FILE_NAME` | 16 | a per-file grant's name, and so `jig remove <package>` and the Remove and Vouch messages | wire: two `START` words |
| `grant_plan::spawnproto` `HOLDER_MAX`, `system_log_protocol` `control::NAME_MAX`, `components/src/timetable.rs` `NAME_MAX` | 32 each | a NameTaken holder, a log registration, an installed program in store mode | wire or runtime |
| `crates/documentation` `PATH_MAX`, magic `CRKRMAN1` | 72 | a manual page's path | on-disk |
| basalt `helpers/packages.py` `NAME_LEN` | 32 | `doc/` member names, a hand copy of nife's value | basalt's own check |
| `vendor/redoxfs` `DIR_ENTRY_MAX_LENGTH` | 252 | a file name on nife's general filesystem | on-disk: a 4-byte node pointer and 252 name bytes make a 256-byte entry |
| `redoxfs_server` `NAME_MAX`, std's `nife` overlay | 4096 (one page) | a name or path on the filesystem request path | runtime |
| `filesystem_protocol::dirent` | 255 | a READDIR record's one-byte length | wire |

`activation_set` and `manifest_note` assume no name width (`manifest_note`'s 8-byte field holds only
the note owner `nife`). U-Boot's 32-byte image name and GPT's 36-unit label are foreign formats and
do not move.

**The premise holds with one correction.** nife's general filesystem does not allow 255-byte names
today. redoxfs stops at 252 and answers 253 to 255 with `EINVAL`. Everything between a program and
redoxfs allows at least a page.

## Fork 1: which bound

- (a) Each of name, version and architecture up to 255 bytes.
- (b) The package's file name as a whole, `<name>-<version>-<architecture>.nifepkg`, within the
  limit, as Debian bounds `name_version_arch.deb` and nothing else.

**Recommendation: (b), with the limit set to 252 bytes, the name limit of the filesystem nife
serves.** Option (a) permits a 775-byte file name that no filesystem in this tree or on Linux can
hold, so the real limit would be the filesystem's anyway, discovered at install time as an
`EINVAL`. Option (b) is one fact checked in one place: the file fits. Each field is then bounded by
it, and a program name, which is its own file under `bin/`, is bounded by the same 252.

**This is a pushback on 255.** calef asked for Debian's number, and Debian's number is Linux's
`NAME_MAX`. nife's is redoxfs's 252. A 253-byte package file name would be valid in the format and
unstorable on nife's own disk. The alternatives:

- 252 (recommended). Matches what nife can store. Three bytes short of Debian; no Debian package
  name is anywhere near it (see prior art).
- 255, and patch the vendored redoxfs to match. Its directory entry becomes 259 bytes, which
  breaks the 256-byte entry's alignment and redoxfs's on-disk format, so every nife redoxfs image
  stops reading upstream's and the other way round. A patch to an on-disk format is the kind §46
  (thin primitives or whole subsystems) says to vendor only when it is needed, and three bytes of
  name is not a need.
- 255 in the format, 252 enforced at install. Two numbers for one fact, which is the shape the
  ladder's first rung exists to remove.

## Fork 2: fixed or variable width

Measured on 2026-10-17 (UTC) against the 131 programs that the three architectures' initial
ramdisks pack between them, mean 13.3 bytes, longest 27. They are every `[[bin]]` in `components/`
and `fixtures/`, plus the `std`, farm, basalt and file-server programs `xtask/src/archive.rs` adds. For packages, the 18 `.nifepkg` assets on basalt's latest release
(`rolling-2026-10-17-155eba5`), 2 to 5 members each, longest file name 30 bytes.

nifefs directory (512-byte blocks; a variable entry is a one-byte length, the name, then the
two `u32`s, 9 bytes plus the name):

| layout | directory bytes for 131 files | blocks | entries in block 0 | files in today's 14 blocks |
|---|---|---|---|---|
| fixed 32 (today) | 5,252 | 11 | 12 | 178 |
| fixed 252 | 34,072 | 67 | 1 | 27 |
| fixed 255 | 34,465 | 68 | 1 | 27 |
| variable, `u8` length | 2,931 | 6 | 22 | about 320 at today's mean, 27 at worst |
| variable, every acronym expanded | 3,095 | 7 | 22 | about 300 |

Holding today's `MAX_FILES` of 178 at a fixed 252 takes 91 directory blocks, 46 KB.

Package header and table of contents (a variable field is a `u16` length and its bytes):

| layout | header + table, 2 members | 3 members | 5 members | cost on the smallest package (21,292 bytes) |
|---|---|---|---|---|
| fixed 32 (today) | 256 | 328 | 472 | |
| fixed 255 | 1,371 | 1,666 | 2,256 | +6.3% |
| variable | 142 to 153 | 204 to 222 | 319 to 320 | smaller than today |

**Recommendation: variable width, in both formats.** It is smaller than today's fixed 32 in every
measured case, and fixed 252 is 6.5 times the directory for nothing. Would I still choose it if
both cost the same work? Yes, and they do not: a fixed field is a one-constant change, and variable
width is the more work. What it costs, so the ruling is made on it:

- Entries are walked, not indexed. Today entry `i` is at `12 + 40i`. A variable directory is
  read front to back. Every reader already loops over all entries, so this changes code, not
  complexity.
- `MAX_FILES` becomes a byte budget. Keep a count ceiling on the header's `count` so a hostile
  header is refused before anything is multiplied by it (`package_archive::MAX_MEMBERS`'s reason),
  and size the directory in bytes.
- `nifefs::ENTRIES_IN_FIRST_BLOCK` stops being a constant. The EL0 block driver in
  `crates/virtio` sees only block 0 of a three-file disk. It becomes "the entries that end inside
  block 0", computed while walking. At a fixed 252 it would be one entry, which breaks that disk
  outright.
- The Kani harnesses over `nifefs`'s parser are rewritten, because the parser is.
- A member name becomes a path, a `u16` length with every `/`-separated component within 252
  bytes, so `bin/<a 252-byte program>` fits. It is one page at most, the bound the filesystem
  request path already enforces.

## Fork 3: whether program names and package names move together

**Recommendation: yes, one constant.** A program is a member of its package (`bin/<program>`), an
entry in the image's archive, and a file on redoxfs. A ceiling that differs across those three is a
program that can be packaged and not installed, which is the failure `package_archive::NAME_LEN`'s
own comment names. Under rule 7 the constant lives in a crate both formats import. `package_archive`
deliberately does not depend on `nifefs`, so neither owns it; which crate holds it is a naming call
for the build lane. basalt's hand copy reads nife's value instead of restating it.

The side channels in the table do not all have to move in the same change, but each needs an
answer:

- **The 16-byte grant name is a defect against this rule today.** `jig remove` refuses a package
  name over 16 bytes, and `mdr` under a flat acronym rule becomes `markdown-renderer`, 17. The
  constant's own comment calls 16 a demonstrator's limit, and lifting it means a frame for the
  caretaker rather than two argument words. That is its own milestone; until it lands, `jig
  remove`'s limit is a recorded exception in the `BUGS` beside it.
- `HOLDER_MAX`, the log registration's `NAME_MAX`, the timetable's `NAME_MAX`, swish's 108-byte path
  and `Label`, and `STEM_LEN` all hold a program or package name, so they read the shared constant.
  A log registration that cuts a name at 32 keeps cutting it until the log protocol is widened,
  which is a wire change and is listed, not bundled.
- `documentation`'s 72-byte path has its own magic and is not a package name. It moves when a
  manual page's path needs to.

## Fork 4: the migration, in order

Who has already acted on the current format:

- basalt has published 17 `rolling-*` Releases in `NIFEPKG1`, the latest holding 18 packages, with
  each package's SHA-256 in its recipe's digest table (milestone 917 (basalt's packages build reproducibly))
  and in the index at `basalt.nifeos.org`.
- nife pins six of those packages by digest in `xtask/basalt-pins.index` and fetches them in CI,
  where a failed fetch fails the row.
- Installed systems: none found. Every nifefs image regenerates from the tree, as §58 (a wider
  archive name) argued, and no `.nifepkg` is known to sit on a disk that outlives a rebuild. The
  maintainers confirm this for the bench boards before step 1 lands.

The two repositories pin each other, so the order matters:

1. nife, one pull request. The shared constant. `nifefs` with variable entries and magic
   `CRKR0003`, every reader in the same change (kernel, `virtio`, `sealed_pair`'s scan, xtask, the
   fuzz targets), since images regenerate and nothing outside nife reads one. `package_archive`
   writes `NIFEPKG2` and reads both 2 and 1. The second reader is there because nife's gates fetch
   basalt's `NIFEPKG1` packages until step 3.
2. basalt. Bump the nife pin to step 1's merge commit, cite it as `nifeos/nife@<sha>`, rebuild
   every package, rewrite every recipe's digest table with `--update-digests`, drop the hand-copied
   `NAME_LEN`, and publish a new Release. Every digest changes, and the pull request says that this
   is why.
3. nife. Copy basalt's new index lines over `xtask/basalt-pins.index` and delete the `NIFEPKG1`
   reader in the same pull request, so the window closes in the change that ends its reason. `jig`
   then refuses a `NIFEPKG1` file with `BadMagic`, and its message says to fetch a rebuilt package.
4. basalt. Re-pin to step 3.

**Sub-fork: does `jig` keep reading `NIFEPKG1` after step 3? Recommendation: no, it refuses.** A
second parser of hostile input kept forever, for archives no installed system holds, costs more than
it saves. The old Releases stay published and unchanged; a reader meets `BadMagic`, not a
misreading, which is §58's reason for a bump.

What else was considered: one atomic nife change with no dual reader, letting the basalt fetch rows
fail until basalt republishes. Refused, because it turns a gate off for the length of a cross-repo
round trip, and the failure it would hide is exactly the one this migration can cause.

## What it unblocks

Measured with a flat rule that spells out every acronym in a name (`abi`, `cpu`, `elf`, `fs`, `gpu`,
`http`, `os`, `pci`, `tls`, `uefi`, `usb`, `uuid`; `mdr` as `markdown_renderer`). The list was taken
from the names' own tokens; foreign program names (`rg`, `mkfs`, `ps`) keep their spelling, as the
acronym test already says. The flat rule itself is pending, and this records only what it would
cost.

| set | names | over 32 today | over 32 expanded | over 28 expanded (`bin/` today) | longest expanded |
|---|---|---|---|---|---|
| programs the images pack | 131 | 0 | 4 | 6 | 41, `pinned_transport_layer_security_exerciser` |
| every program in the tree | 132 | 1 | 7 | 9 | 44, `unified_extensible_firmware_interface_loader` |
| crates (Cargo's own limit, not nifefs's) | 124 | 2 | 8 | 12 | 44 |
| packages | 38 | 0 | 0 | 0 | 17, `markdown-renderer` |

The four image programs it unblocks: `usb_keyboard_driver` (36), `current_cpu_reader` (38),
`os_primitives_benchmarker` (39) and `pinned_tls_exerciser` (41). `gpu_driver` and `uuid` land at
29 to 32, which fits the archive and not a package's `bin/` member. Nothing expanded passes crates.io's 64.

## Prior art

Read on 2026-10-17 (UTC) from each project's own source, not recalled.

| ecosystem | bound | what is bounded | source |
|---|---|---|---|
| Debian Policy | none (at least 2 characters) | package, version, architecture | Debian Policy sections 5.6.1 and 5.6.7 |
| dpkg | none | `pkg_name_is_invalid` checks emptiness, first character and charset only | `lib/dpkg/parsehelp.c` |
| dak (ftp-master) | none | `re_field_package` is unbounded | `daklib/regexes.py` |
| lintian | 80, a warning | the whole `.deb` file name, with the longest architecture substituted (`package-has-long-file-name`) | `lib/Lintian/Check/Archive/File/Name/Length.pm` |
| crates.io | 64 characters; version 150 | name | `crates_io_validation/src/lib.rs` |
| npm | 214 | name, for new packages | `validate-npm-package-name` `lib/index.js` |
| RubyGems.org | 255 | name | `app/models/rubygem.rb` |
| Nix | 211 | store path name | `libstore/include/nix/store/path.hh` (`MaxNameLen`) |
| PyPI, pacman, RPM, Go modules | none found | name | warehouse `models.py`; `pkgname.sh.in`; `parsePreamble.cc`; `module/module.go` |
| Linux, ext4, Btrfs | 255 bytes | a file name | `NAME_MAX`, `EXT4_NAME_LEN`, `btrfs_tree.h` |
| APFS | 255 characters, not bytes | a file name | measured on macOS: 255 ASCII accepted, 256 refused, 255 three-byte characters (765 bytes) accepted |
| redoxfs upstream | 252 bytes | a file name | `src/lib.rs` `DIR_ENTRY_MAX_LENGTH` |

So Debian's 255 is a filesystem's limit and not a packaging rule; the only packaging check on the
file name is lintian's warning at 80.

What Debian actually uses, from sid main amd64's `Packages.xz` fetched 2026-10-17 19:36 (UTC),
76,655 binary packages:

| field | median | p99 | p99.9 | longest |
|---|---|---|---|---|
| package name | 17 | 40 | 49 | 75 |
| `.deb` file name | 36 | 66 | 77 | 102 |
| version | 8 | 28 | 37 | 52 |

Four binary package names pass 64, so crates.io's cap would refuse real Debian packages; none passes
128. 252 leaves Debian's longest file name more than twice its length in headroom. Not checked:
Homebrew, Alpine's apk and abuild, Cargo's local `restricted_names.rs` (searched, not audited), and
Fedora's packaging guidelines as distinct from `rpm` itself.

## What is blocked until this is ruled

Any program or package name past 28 bytes, so the flat acronym rule's renames. The build lane for
steps 1 to 4. The `jig remove` grant milestone, which waits on Fork 3's constant.

## BUGS

- The measurement counts names in the tree on 2026-10-17 (UTC). The per-architecture archives pack
  subsets of the 131, so the directory figures are an upper bound for any one image.
- APFS's limit was measured on one macOS host; no Apple document stating it was found.
