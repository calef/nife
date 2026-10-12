# Prior art: an appendix to milestone 533

[Milestone 533 (whether the roadmap should become issues, and what a citation means after the repository
split)](../0533-the-roadmap-after-the-split.md) keeps the question, the options and the recommendation.
This appendix holds the proposal's reading of how eight other projects keep a durable record, mint
an identifier and use a tracker, with what that reading says about this tree.

## Read rather than recalled

Fetched 2026-09-19. Every quotation below was read from the URL given.

| Project | Durable record | Identifier | Minted | Tracker's job |
|---|---|---|---|---|
| **Rust RFC** | in-tree `text/NNNN-slug.md` in `rust-lang/rfcs` | number **and** slug | the **pull request number**, at PR-open | a tracking issue in a *different* repo holds implementation status |
| **Go proposal** | in-tree `design/NNNN-shortname.md` | number and shortname | the **GitHub issue number** | the issue **is** the identity and holds the state |
| **Kubernetes KEP** | in-tree `keps/sig-x/NNNN-short-name/README.md` + `kep.yaml` | number and short name | the **tracking issue number** | issue is the breadcrumb; `kep.yaml` holds status and `replaces`/`superseded-by` |
| **Fuchsia RFC** | in-tree `//docs/.../rfcs/`, indexed by `_rfcs.yaml` | `RFC-NNNN` | by the Eng Council **at acceptance** | a filed bug, linked from the RFC header, for implementation |
| **Python PEP** | in-tree rST in `python/peps` | number | by PEP editors at draft approval | separate; routine work never becomes a PEP |
| **Debian** | `debian/changelog` in the source package | global bug number, package binding is a mutable field | the BTS | the BTS is the record; the tree references it by string |
| **FreeBSD ports** | `Makefile`, `distinfo`, plus tree-wide `UPDATING` and `MOVED` | port origin path (`category/name`) | by the tree | Bugzilla PR number, referenced from commits by string |
| **Homebrew** | formula file in a tap | **`user/repo/formula`**, a path-shaped name | by the tap | issues per tap |

Four things this table says that are worth saying out loud.

Every one of the eight keeps the durable record in a versioned tree. Not one of them put the
argument in the tracker. The projects that use a tracker heavily (Go, Kubernetes) use it for the
*number* and the *state*, and still check the design document into a repository. That is a strong
answer to question (1), and it is unanimous.

The dominant pattern is that the tracker mints the identifier and the tree holds the record.
Rust: *"Don't assign an RFC number yet; This is going to be the PR number and we'll rename the file
accordingly if the RFC is accepted."* Go: *"A proposal is a suggestion filed as a GitHub issue,
identified by having the Proposal label,"* and *"The design doc should be checked in to the proposal
repository as `design/NNNN-shortname.md`, where `NNNN` is the GitHub issue number."* Kubernetes:
*"KEPs are now prefixed with their associated tracking issue number. This gives both the KEP a
unique identifier and provides an easy breadcrumb."*

This tree already opens a claim pull request as a lane's first act (§90), and GitHub already mints a
unique never-reused number for it. The hand-minted contiguous number that two sessions collided on
three times in one day is a number GitHub was already giving us and we were throwing away. That is
a real option, and it is listed as such in the parent's BUGS rather than recommended. PR numbers
are per-repository and therefore have option B's problem after the split. And reconciling 441
existing blocks with a PR counter now past 999 is the dangerous edit of §194 (a renumber is the price) at full scale.

Every cross-reference mechanism in the table is a plain string convention, not a link. Debian's
deb-changelog(5): *"If this upload resolves bugs recorded in the distribution bug tracking system,
they may be automatically closed on the inclusion of this package into the distribution archive by
including the string: `Closes: #nnnnn` in the change details."* FreeBSD: *"Many FreeBSD changes refer
to the PR number that prompted the change."* Homebrew qualifies across taps by path:
*"Use the fully qualified name to select the formula from a particular tap: `brew install vim` /
`brew install username/repository/vim`."* Nothing structural survives a split; a grep-able string
does. Option C is that observation applied to this tree.

A project that moved, with its reason, since the brief asked for one. LLVM moved from Bugzilla
to GitHub Issues, and the RFC's stated reason is entirely about the tool, not about where records
belong: *"Our bugzilla installation is...not great. It's been not-great for a long time now,"* and
*"Importantly, Github Issues is significantly less user-hostile than our bugzilla is, for new
contributors and downstream developers who just want to tell us about bugs!"* The move is
tracker-to-tracker for bug reports from strangers. That is the first item in the "genuinely better
at" list of [`tracker-checks.md`](tracker-checks.md), and not the roadmap.

A correction to this lane's own research, recorded because this tree carried a fabricated block
quote for twelve days. A first pass reported that the LLVM RFC discussed preserving `PRxxxxx`
numbers already baked into source comments, which would have been a direct precedent for the 3,148
sites here. Fetching the page found no such discussion. The claim is withdrawn and is not used
anywhere in the proposal. Rust's move in the opposite direction is also worth one line since it was checked.
The compiler team's MCP process put a *lighter* tier of proposal into tracker issues while leaving
the heavy tier in the RFC repository. That is a proportionality argument rather than a medium one.
