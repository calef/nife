# CI's toolchain download, 2026-10-19 (provisional name)

Measured 2026-10-19 (UTC) by maintainer/rustup-retry, after a docs-only pull request (#2379) went red
because one TCP connect to static.rust-lang.org timed out. Two questions: how often does the
download fail CI, and should the toolchain be cached? The constraints are those of
[actions-cache-budget-2026-10-07.md](actions-cache-budget-2026-10-07.md) and calef's rulings on
#1814.

## 1. How often the download fails

Every failed job from 2026-10-04 to 2026-10-19 00:00 in the ten workflows that install a toolchain
(ci.yml, verify.yml, mutation-gate.yml, stick-maker-hosts.yml and six others), including the earlier attempts of runs
that were rerun: 423 job logs from 343 runs, searched for rustup's download errors.

| When (UTC) | Job | What failed | Cost |
|---|---|---|---|
| 2026-10-08 01:38 | clippy, merge group of #1843 | checksum mismatch on the floating `nightly` manifest, the CDN mid-publish | ejected from the queue; requeued 14 minutes later and merged |
| 2026-10-18 23:52 | swish-check-x86_64, #2379 | TCP connect timed out (30 s) on the pinned manifest's `.sha256` | one rerun |

Two in two weeks, out of about 3,300 pull request and merge-group runs of ci.yml. Both cleared on the
next attempt. rustup made one attempt each time.

The 2026-10-08 one came from a toolchain no job needed. Five ci.yml jobs ran `rustup toolchain
install nightly`, which is the floating channel, not the pin; cargo then fetched the pin on first use
outside any step that could retry. The floating install cost a median 9.9 s per job (n=129).

## 2. What changed

`helpers/rustup-retry.sh` runs rustup up to four times, waiting 10, 30 and 90 s, and only when the
output looks like a download failure. A real error fails at once. On the last failure it prints the
URL and the attempt count, and in Actions an `::error` annotation. `script/bootstrap` and every
direct rustup step in ci.yml, mutation-gate.yml and stick-maker-hosts.yml go through it, and the
ci.yml steps install the pin by `toolchain install` with no name. `helpers/rustup-retry-selftest.sh`
in `script/lint` holds it against a stub rustup, and was watched failing with the classifier broken
and with the bound removed.

Proved by hand with `RUSTUP_DIST_SERVER=http://127.0.0.1:9` and a scratch `RUSTUP_HOME`: four
attempts, then `failed 4 of 4 attempts fetching http://127.0.0.1:9/dist/2026-10-18/channel-rust-nightly.toml.sha256`
and exit 1. A non-download error (`--no-such-flag`) failed after one attempt.
