---
status: NOT-STARTED
raised: 2026-10-13
milestone_dependencies: 40, 810, 809
decision_dependencies: 197, 297, 208, 229
machine_requirements: none
specific_machine: none
needs_person: yes
---
# 918. A package carries its program's documentation, and `jig` installs it into the documentation store

*Name: provisional. The number 918 is provisional until the merge queue lands it, and the title
and the slug are drafts. Branch `lane/docs-ship-in-packages`.*

calef, 2026-10-13 (UTC): *"I see there is no documentation installed with ripgrep. We have a
documentation system. We should use it to install docs with the programs."*

And the same day: *"Can we ensure that documentation is included in future ports of packages?"*

Under §297 (software reaches a nife system as a package `jig` installs from basalt's index, the
kernel included), that covers every package. `ripgrep` is the first.

## Where it starts

- Milestone 40 (documentation as a system service: searchable, rendered, and installed by
  packages) is BUILT. Its design says a doc bundle is part of a package: markdown pages plus a
  small index shard, installed into a documentation store with the component. The index is a merge
  of shards, one per installed package.
- What milestone 40 built is a host-built store. `cargo xtask manual` writes `doc/<bundle>/` and
  `doc/bundles` into the image from `xtask/src/manual.rs`'s `DOC_BUNDLES` table. The store's
  layout is `documentation::index::STORE_DIR`, `MANIFEST` and `SHARD`.
- §197 (a package is one archive file) lists "Documentation bundle and index shard" among what a
  package must carry, citing milestone 40.
- `crates/package_archive` knows one member convention: `bin/<program>` (`PROGRAM_PREFIX`). Any
  other member is carried and ignored. A package with `doc/` members therefore installs today, and
  its pages are dropped.
- `jig` is the client. The progenitor is the installer (§208 (installing a package is granting it,
  and the installed programs are versioned)). Its `install` places the program at
  `packages/<name>/<version>/<program>` and commits a new generation. `remove` and `rollback`
  commit generations too.
- A bare name reaches a program through the live generation (§229 (how a bare name at the prompt
  reaches an installed program)). The documentation store does not consult the generation at all.
- `apropos <word>` is a swish builtin (`components/src/swish/prompt.rs`). It reads `doc/bundles`
  and searches each bundle's `index` shard. It prints a typeable location such as
  `doc/swish/pipes.md`, and `mdr <location>` renders that page. Milestone 279 (documentation search
  is a shell builtin, so a second shell has none) moves the search behind a service. This milestone
  feeds whichever reader is current.
- basalt's `recipes/ripgrep.toml` names one member, `bin/rg`. `helpers/packages.py` packs it with
  nife's `cargo xtask package`.

### What upstream ships, checked 2026-10-13 (UTC)

The `ripgrep` 14.1.1 crate, whose SHA-256 matches the recipe's `sha256`:

- `GUIDE.md` (40,701 bytes) and `FAQ.md` (41,724 bytes), plus `README.md` and `CHANGELOG.md`.
- No man page file. `crates/core/flags/doc/template.rg.1` is a man(7) template, and
  `rg --generate man` fills it from the flag definitions. Producing the page needs an `rg` that
  runs on the build host.
- The template uses eleven man(7) requests: `TH SH SS PP sp IP TP RS RE EX EE`, and the font
  escapes `\fB \fI \fP`.
- `FAQ.md` uses raw HTML headings (`<h3 name="config">`). `crates/documentation` passes HTML
  through as text, a limitation its own `BUGS` records.

## The work

1. A member convention for documentation. A package carries its bundle as members under
   `doc/`: each page as `doc/<page>.md`, and the shard as `doc/index`. The bundle's name is the
   package's name. `package_archive` gains a `DOC_PREFIX` beside `PROGRAM_PREFIX`, and a function
   that yields the bundle's members. A member name is at most `NAME_LEN` (32) bytes, which bounds a
   page's name to 26.
2. The shard is built when the package is built. Milestone 40 chose that shape because the
   system has no directory enumeration and indexing needs none. `xtask` gains a verb that builds a
   bundle and its shard from a list of markdown files, with the builder `cargo xtask manual` uses.
   `cargo xtask package` takes those as members.
3. Installing makes the pages findable. When a generation that holds a package goes live, its
   bundle appears as `doc/<package>/` and `doc/bundles` names it. When the package leaves the live
   generation, both go. The installer places the bundle beside the program, under
   `packages/<name>/<version>/doc/`, so `rollback` can restore the pages with the program.
   `apropos` and `mdr` find a newly installed package's pages without an image rebuild.
4. The store follows the live generation. `install`, `remove` and `rollback` each rewrite
   `doc/bundles` from the image's own bundles plus the live generation's packages. A page is
   findable exactly when its program runs by bare name.
5. basalt builds the bundle from the recipe. The recipe names the upstream documents to ship
   and how the reference page is made. `helpers/packages.py` produces the pages and passes them to
   `cargo xtask package`.
6. `ripgrep`'s bundle. It ships `GUIDE.md` and `FAQ.md` as upstream wrote them. It adds
   `rg.md`, converted at package build time from `rg --generate man`.
7. basalt refuses a package with no bundle. `helpers/packages.py` fails the build of any
   recipe whose package carries no `doc/` member. A recipe may opt out with an explicit exception
   and its reason, in the shape `deny.toml`'s `reason =` entries use:
   `documentation = { exception = true, reason = "..." }`. Every key is provisional. The check
   lands in the same pull request as `ripgrep`'s bundle, so `ripgrep` never fails it.
8. A port's exit names its documentation. §297's item 3 says a port's exit names the program
   installed with `jig` from basalt. This milestone adds the pages to that exit: after
   `jig install <package>`, `mdr doc/<package>/<page>.md` renders and `apropos` finds it. basalt's
   README, in its section on adding a recipe, and `notes/packages.md` state it beside the
   recipe shape. The rule in §297 itself is calef's to amend, and the building lane
   asks him, quoting his words above.

## How a man page reaches a markdown store

The store holds markdown, by milestone 40's design. A man page is roff, so something converts it.

The recommendation: convert at package build time, and ship upstream's markdown too. basalt builds
`rg` for the build host from the same pinned source, runs `rg --generate man`, and converts the
output with pandoc (`-f man -t commonmark`). pandoc also reads mdoc, which BSD-derived programs use.
Upstream's own markdown ships unchanged. A host-side check renders every page with
`crates/documentation` and fails the build on a truncated line.

Measured on the template with pandoc 3.12.1: headings, bold and italic come through. pandoc
escapes brackets (`\[*OPTIONS*\]`), and the renderer shows backslash escapes literally today. The
renderer learns CommonMark backslash escapes as part of this milestone. That fix is owed to any
upstream markdown anyway.

The alternatives, and why each is not recommended:

- mandoc `-T markdown`. It reads mdoc only. On this Mac, `mandoc -man -T markdown` answers
  "-Tmarkdown unsupported for man(7) input", and ripgrep's page is man(7).
- A converter of nife's own in `xtask`, for the man(7) subset ripgrep uses. It is eleven requests
  today. Every later port brings a wider subset, or mdoc, and the converter grows into a roff
  parser. The tree's habit is to take the parser and write the renderer (milestone 40's reuse
  table).
- Ship roff and render it on the device. That puts a second format in the store and a roff reader
  in a confined program. Milestone 40's design borrowed the man architecture "minus the troff".
- Ship upstream markdown only. `GUIDE.md` and `FAQ.md` do not list the flags. The reference a
  person reaches for is the flag list.
- `rg --help` output inside a code fence. It needs the same host build. It loses the man page's
  sections and emphasis, and nothing indexes a fenced block's words as headings.

pandoc is a build-time tool on basalt's runners. It enters no image and no package, only the
markdown it writes. It is still a dependency, and an architect rules on each one (§46 (thin
primitives or whole subsystems; we write everything in between)). The building lane asks calef
before basalt's build takes it.

## Relation to milestone 684

Milestone 684 (a documentation site, and documentation that ships in packages) asks which of
nife's own documents go to a site, which ship on the device, and which stay project records. This
milestone does not answer that. It builds the mechanism a shipped document uses, and it ships
program documentation, which is not in 684's question. When 684 is answered, each document it sends
to the device travels as a bundle in this shape.

## Ports already planned

Port milestones already amended under §297 include 826, 834, 99, 170, 171, 258, 905, 906, 907
and 897. They gain the documentation requirement through basalt's refusal in item 7, not through
edits to each block. A port reaches its exit through basalt's build, and the build refuses it
without a bundle or a recorded exception. A block whose Done-when should also name the page is
amended by the lane that starts it.

## First-party programs

swish, the process tools, `kilo` and every other first-party program reach a system as packages
under §297. Their bundles move into those packages the same way, and `DOC_BUNDLES` retires.
Milestone 909 (basalt composes the bootable image from its packages) owns that move. The composed
image's store is then the merge of its packages' bundles. Which notes become bundles is 684's.

## Done when

On aarch64, riscv64 and x86_64, under QEMU, in `script/swish-check` or basalt's distribution
stage (milestone 916 (integration tests live in basalt)):

- After `jig install ripgrep`, `apropos ripgrep` lists `doc/ripgrep/rg.md`, and `mdr
  doc/ripgrep/rg.md` renders its NAME section.
- After `jig remove ripgrep`, `apropos ripgrep` finds no ripgrep page, and `mdr doc/ripgrep/rg.md` is
  refused as not found.
- After `jig rollback`, both work again.

No image is rebuilt between those lines.

## Dependencies

- Milestone 40 is the store, the renderer, the shard format and its builder.
- Milestone 810 (`ripgrep` is packaged in basalt and installed with `jig`) is the package and its
  build.
- Milestone 809 (the package client becomes a program, `jig`, with the verbs an index needs)
  supplies `install`, `remove` and `rollback`.
- Reproducible package builds (`lane/reproducible-packages`, not yet landed) must cover the
  bundle, since a converted page has to be the same bytes on every runner. That milestone joins
  this field when it lands.
- §197 is the container, and its metadata table already lists the bundle.
- §297 makes this apply to every package.
- §208 and §229 make the live generation the source of what is installed.

Reuse: `crates/documentation`'s builder and reader unchanged in format, `package_archive`'s member
table, the installer's `place_program` path, and basalt's package build.

## BUGS

- Publishing the first package with a shard freezes the shard layout (`CRKRMAN1`, `VERSION` 2).
  Today the layout is a build artifact nothing outside the image holds. Once a published package
  carries one, a layout change needs a reader that knows both versions, or every package rebuilt.
  The layout is a format two programs agree on, so the building lane puts it to calef before that
  first publish.
- `FAQ.md`'s raw HTML headings render as text until the renderer handles them. The build-time
  check reports them rather than failing on them.
- Each architecture's package carries the same bundle bytes. The pages are architecture-free, and
  three copies are published.
- A page name is at most 26 bytes, from the member name limit.

## Follow-on

- **Milestone 909.** Composing the image's documentation store from its packages' bundles, and
  retiring `DOC_BUNDLES`. Its dependency field gains 918 when this lands.
- **Milestone 684.** Which of nife's notes ship on the device.
- **Milestone 279.** The search service reads the same `doc/bundles` this milestone maintains.

## Index row

A package carries its program's documentation as a bundle with its index shard, and installing,
removing or rolling back the package makes its pages findable or not with `apropos` and readable
with `mdr`, with no image rebuild. `ripgrep` ships first: its guide, its FAQ, and its man page
converted to markdown at package build time. basalt refuses a package without a bundle unless its
recipe records an exception and its reason.
