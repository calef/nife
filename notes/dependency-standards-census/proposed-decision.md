# Proposed decision: which of nife's standards a dependency is held to

Status: proposed, for an architect. Provisional; the integrator mints a section number if it is
ruled. The data is [the census](../dependency-standards-census.md), measured 2026-10-18 and
2026-10-19 (UTC) over the 99 crate versions nife ships, its two vendored trees and the milestone 784
candidates.

§46 decides whether nife takes a dependency. Nothing says what a taken dependency must be. Today
the only written bar is `deny.toml` through `script/supply-chain`: licenses, advisories, the
unmaintained list, yanked versions, crates.io as the only source, and one version per crate. This
proposal takes nife's internal standards one at a time and asks which should hold dependencies too.
The questions are ordered by how much each answer changes.

The pattern in the data, and in every organization checked: a standard that describes a crate's
risk transfers to dependencies. A standard that describes how its owner works does not. Unsafe
soundness, advisories and maintenance are risks. Size limits, mutation scores, proofs and BUGS
sections are process. Google, Chromium, Fuchsia, Mozilla's cargo-vet, Android, the Rust project's
tidy and the Linux kernel all gate dependencies on unsafe review and supply chain. None requires
mutation testing, Miri, fuzzing, proofs, coverage or length limits of code it does not own. Sources
are listed at the end.

## Q1. Is #2379's confidence bar a gate, a goal, or only for crates nife publishes?

Pull request #2379 proposes that any crate nife builds on meet `nifeos/crate-workflows`' gates. The
bar has five parts: Kani with every harness falsified, a mutation baseline, semver checks, a tested
minimum Rust version, and a stranger build.

- Passing today: 0 of 99 shipped. One runs Kani (zerocopy), none has a falsification record, and
  none runs mutation testing. Nine run semver checks.
- Cost as a gate: every shipped dependency goes, including the kernel's three, the network stack
  and the TLS stack. Nothing could replace them that meets the bar either.
- Prior art: no organization found applies anything like it to dependencies. The one formal effort
  on code its users do not own, verify-rust-std, is a funded contest on `std`.

Options:

- (a) A gate on every dependency. Not satisfiable by any crate measured.
- (b) A goal: a published list of where each dependency stands against the bar, with no consequence
  for missing it.
- (c) The bar applies to crates nife publishes and to mechanisms that leave nife, as crate-workflows
  already enforces. A dependency nife consumes is held to Q2 to Q7 instead.

Recommendation: (c). The bar describes nife's own process, and the census shows the ecosystem does
not follow it. (b) would publish a list on which every row fails, which informs no decision.

## Q2. How is a dependency's unsafe code held to account?

nife holds its own unsafe code three ways. Each `unsafe` block carries a SAFETY comment
(`clippy::undocumented_unsafe_blocks`). Each `unsafe fn` carries a `# Safety` section (milestone
112). Density stays under a ceiling of 60 blocks per 10,000 code lines outside `kernel/src/arch/`
(nife is at 57).

- Passing today: density 65 of 99, SAFETY comments on every block 52, `# Safety` on every unsafe fn
  72. 44 have no unsafe at all.
- Cost as a gate: the density ceiling alone fails 34. Those include the SIMD block ciphers, `spin`,
  `heapless`, `smallvec`, `zerocopy`, `getrandom`, `httparse` and `tock-registers`. Most of the
  cryptography and both kernel register crates would go. The ceiling measures what a crate does
  (SIMD, atomics, MMIO), so a replacement doing the same job would fail it too.
- Prior art: this is the one standard everyone applies to dependencies. cargo-vet's
  `safe-to-deploy` has a reviewer reason about every unsafe block, at admission and again on each
  version delta. Google grades soundness `ub-risk-0` to `ub-risk-4`, and Fuchsia requires
  `ub-risk-2` by default and `ub-risk-1` for cryptography and untrusted input.
- Audits nife could import today: the five public cargo-vet sets reach the exact version of 23 of
  99 rows. That leaves 76 for nife to review or exempt on the record.

Options:

- (a) Gate on nife's own density ceiling and SAFETY rule. Removes 50 of the 99 with no
  replacement that passes.
- (b) A recorded unsafe review per crate version, at admission and on each update, kept as
  cargo-vet audits in the tree. Import Google's, Mozilla's, Bytecode Alliance's, ISRG's and Zcash's
  sets, and grade with Google's `ub-risk` levels. A crate nife has not reviewed carries an exemption
  that says so.
- (c) Measure and report the census numbers weekly, with no review record.

Recommendation: (b). It is the only option that records a judgment about each crate, and it uses
the criteria the largest Rust users already share. It would still be the choice if (c) cost the
same. The cost is the review of 76 crate versions, plus a delta review on each update after that.

## Q3. Is code the kernel links held to the kernel's own bar?

The kernel runs unconfined, and §46 says to write the kernel. It links three crates: `aarch64-cpu`
11.2.0, `spin` 0.12.3 and `tock-registers` 0.10.1. Together they hold 71 unsafe blocks.

- Measured: the text check finds SAFETY comments on 34 percent of `spin`'s blocks and on none of
  the other two's. `spin` and `tock-registers` pass their tests and run clean under Miri.
  `aarch64-cpu`'s doctests do not build on a host. `aarch64-cpu` and `tock-registers` are in none of
  rustc, Chromium, Android or Fuchsia.
- Cost of holding them to nife's bar: either vendor all three under §46's patch rule and add the
  missing SAFETY comments, or write the parts the kernel uses. About 9,000 code lines in total, of
  which the kernel calls a fraction.
- Prior art: Fuchsia and Chromium both tighten review for code with privilege. Fuchsia asks
  `ub-risk-1` for platform-security crates, and Chromium keeps crates that fail its bar out of
  privileged processes.

Options:

- (a) Kernel-linked code meets nife's own unsafe rules in full, so vendor or write it.
- (b) Kernel-linked code meets Q2's review at the strictest grade (`ub-risk-1`), without vendoring.
- (c) No separate tier: Q2 applies as it does everywhere else.

Recommendation: (b). The record is what matters. A strict grade on three small crates is cheap and
reviewable. (a) forks three maintained crates to add comments. With equal effort the
recommendation would still be (b), because a fork's maintenance cost is ongoing and the comments
add no check a `ub-risk-1` review lacks. This fork affects §46's "write the kernel", so it is the
architect's call.

## Q4. Must a dependency's own tests run?

nife gates its own code on tests that pass, with 85 percent line coverage per file.

- Passing today: 76 of 99 packages build and pass their own suite as published. 93 have upstream CI
  that runs tests. The 23 failures are packaging: missing fixture files, dev-dependency mismatches,
  newer lints and host-only doctests. 11 packages run zero tests.
- Cost as a gate on the published package: 23 crates need either a repository checkout to test from
  or a recorded exemption, among them `rustls`, `rustls-webpki` and `smoltcp`.
- Prior art: Google's third-party policy requires a crate's unit tests in its presubmit, the
  strongest test rule found. Others do not run dependency tests.

Options:

- (a) Admission requires upstream CI that runs tests. Today `managed`, `num-bigint-dig`,
  `stable_deref_trait` and `untrusted` fail, having no CI that has ever run.
- (b) (a), plus nife runs every shipped dependency's tests weekly from its source, as a report.
- (c) Neither.

Recommendation: (b). The weekly run would have shown the census's numbers without a lane, and it is
the cheapest way to see a regression in a pinned version. Coverage is not recommended for
dependencies. No organization requires it, and measuring it needs a per-crate instrumented build.

## Q5. Does Miri run on dependencies?

nife runs Miri weekly over its own host crates (milestone 79). That run is a report, not a required
check.

- Measured: 31 of 99 run Miri in upstream CI. nife ran `cargo miri test` on 17 crates chosen for
  their unsafe code, and all ran clean. Miri checks only the paths a package's tests reach.
- Cost: a weekly run over the 55 shipped crates with any unsafe code. The 17 took 26 minutes in
  total, most of it `lz4_flex`.
- Prior art: none requires it of dependencies. Fuchsia documents Miri for its own code.

Options:

- (a) nife's weekly Miri run covers every shipped dependency with unsafe code, as a report, like its
  own.
- (b) Admission requires upstream Miri. Would fail 68.
- (c) Neither.

Recommendation: (a). It costs one more job on a schedule nife already runs, and it is the only check
here that looks inside a crate's unsafe code by machine.

## Q6. Are dependencies that parse untrusted input fuzzed?

nife fuzzes its own parsers of bytes it did not write, 60 seconds per target, as a required check
(milestone 42).

- Measured: 9 of 99 have upstream fuzz targets, and three have their own OSS-Fuzz project
  (`httparse`, `lz4_flex`, `rustls`). The shipped crates that parse bytes from the network or a
  disk include `httparse`, `rustls`, `rustls-webpki`, `der`, `smoltcp`, `lz4_flex` and RedoxFS.
- Prior art: none requires it of dependencies.

Options:

- (a) A nife fuzz target for each shipped dependency that parses untrusted input, unless upstream
  fuzzes it continuously (OSS-Fuzz). That would add targets for `smoltcp`, `rustls-webpki` and
  `der`, and check that nife's own targets already reach RedoxFS through its server.
- (b) Neither.

Recommendation: (a). nife's thesis is confining code it did not write, and these crates are where
hostile bytes enter. This recommendation is about risk, not effort. Each target is a few dozen
lines.

## Q7. What makes a dependency maintained enough?

§198 refused `rustls-rustcrypto` on maintenance: an alpha release, 17 months stale, two of three
versions yanked, and pinned superseded majors. cargo-deny's `unmaintained = "all"` fires only when
RustSec has published an advisory.

- Measured: three repositories are archived (`cfg-if`, `lazy_static`, and `spin` 0.9.9's old home),
  and cargo-deny says nothing about them. 7 crates have had no release in 24 months, and several are
  finished rather than abandoned (`byteorder`, `subtle`, `untrusted`). 15 have one crates.io owner,
  four of them the Rust project's account.
- Prior art: the Linux kernel asks for a named person committed to each crate. Chromium and Google
  cap versions and ban crates with known defects. OpenSSF Scorecard automates checks of this kind.

Options:

- (a) Admission and each update record §198's criteria for the crate: release maturity, time since
  release, yanked history, owner count, archived state. A reviewer judges; no number is a gate.
- (b) Gate on an archived repository or no release in N months. Today that fails 4 rows on archive,
  and 7 more at N = 24.
- (c) Leave it to RustSec's unmaintained advisories.

Recommendation: (a). The measured cases show finished crates and abandoned ones look the same to a
date check. A recorded judgment can tell them apart; a threshold cannot.

## Q8. Do nife's size and documentation rules apply to dependencies?

These are the file limit of §266 (a Rust source file stays under 2,000 lines), the function limit
of §283 (a function is held to 100 code lines by a lint ratchet), and §267's 40-line ceiling on a
comment block. With them go the BUGS-section practice and clippy at `-D warnings`.

- Passing today: files 84 of 99, functions 77, comment blocks 33, BUGS heading 0. Clippy was not
  measured, because its lints are capped for dependencies. 79 run clippy upstream.
- Cost as gates: the file and function rules remove `rustls`, `smoltcp`, `curve25519-dalek`,
  `libc`, `zerocopy` and `typenum` among others. The comment-block rule fails 66, mostly on
  crate-level rustdoc.
- Prior art: no organization applies size limits to code it does not maintain. Chromium tells
  reviewers to skip tests, benchmarks and examples, and treats size only as review cost.

Options:

- (a) Apply them to dependencies.
- (b) They are rules for code nife maintains: its own and the patches it carries on vendored trees.
  They do not apply to upstream code.

Recommendation: (b). They exist so nife's code stays readable to the people who change it, and nife
does not change its dependencies.

## What the recommendations mean for milestone 784

Under (b) for Q2 and Q7, each candidate needs a recorded unsafe review and a maintenance judgment:

- `r-efi` 7.1.0: 3 unsafe blocks to review, one owner, no audit to import. It enters the boot chain,
  and Q3 does not cover it as written; the loader links nothing else.
- `vte` 0.15.0: 44 unsafe blocks with `arrayvec`, and one commit upstream in 12 months.
- `pulldown-cmark` 0.13.4: 1 unsafe block, continuous fuzzing upstream.
- `plist` 1.10.1: 14 crates for a host program, 7 of them from repositories where one person wrote
  over 80 percent of the commits.

## Sources

- cargo-vet criteria and imports: <https://mozilla.github.io/cargo-vet/built-in-criteria.html>,
  <https://mozilla.github.io/cargo-vet/importing-audits.html>
- Google's audit criteria: <https://github.com/google/rust-crate-audits/blob/main/auditing_standards.md>;
  Google's third-party Rust policy: <https://opensource.google/documentation/reference/thirdparty/rust>
- Chromium: <https://chromium.googlesource.com/chromium/src/+/main/third_party/rust/OWNERS-review-checklist.md>,
  <https://chromium.googlesource.com/chromium/src/+/main/docs/security/rule-of-2.md>
- Fuchsia: <https://fuchsia.dev/fuchsia-src/development/languages/rust/external_crates>,
  <https://fuchsia.dev/fuchsia-src/development/languages/rust/unsafe>
- Android: <https://android.googlesource.com/platform/external/rust/android-crates-io/+/refs/heads/main/README.md>
- Rust's tidy: <https://github.com/rust-lang/rust/blob/master/src/tools/tidy/src/deps.rs>
- Linux kernel: <https://rust-for-linux.com/third-party-crates>
- verify-rust-std: <https://model-checking.github.io/verify-rust-std/>
- OpenSSF Scorecard: <https://github.com/ossf/scorecard/blob/main/docs/checks.md>
