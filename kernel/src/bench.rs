//! Microbenchmarks over the paths a microkernel lives on (milestone 21).
//!
//! Compiled in only by `--features bench` (`script/bench`); the bench boot diverges here before
//! the milestone tour, runs each benchmark in a fixed order, prints machine-readable lines, and
//! **halts**. It never semihosts: under HVF the semihosting `hlt` traps to the guest instead of
//! exiting (see xtask's `test()`), so the contract is output-based in both modes: `xtask bench`
//! owns the QEMU process, watches for `bench: done`, and terminates it. One exit mechanism,
//! accelerator-independent.
//!
//! # The two instruments (design/roadmap/0021-benchmarks.md)
//!
//! - **icount (default):** QEMU virtual time is a deterministic function of instructions
//!   executed, so these counter deltas are *exact and reproducible per binary*: the same kernel
//!   prints the same numbers every run. But they are NOT stable across *different* binaries: adding
//!   unrelated live code shifts even untouched benchmarks by several percent, non-uniformly, because
//!   the compiler remakes whole-crate inlining and monomorphization decisions (notes/benchmarks.md
//!   has the measurement). So `bench/baseline-aarch64.txt` + `--check` is a **coarse tripwire** (10%) for a
//!   gross regression, not a fine attributor. Magnitudes are fiction anyway (TCG models no caches);
//!   the `--real` medians are the fine signal.
//! - **HVF (`--real`):** the kernel runs natively on the host core; real caches, real TLBs, the
//!   hardware counter at its real frequency. Magnitudes are true, determinism is gone (a shared
//!   desktop machine underneath), so real runs report and never gate.
//!
//! # Reading the numbers
//!
//! Each line is `bench: <name> <counter_ticks> <iters>`. The counter is `CNTVCT_EL0` at
//! `CNTFRQ_EL0` Hz (printed first), so ns/iter = ticks * 1e9 / freq / iters; xtask does the
//! division. Warmup iterations run untimed before each measurement so thread spawn and first
//! rendezvous costs land outside the window.
//!
//! **Every program it enters is measured first** (milestone 563 (a seal check that reads bytes
//! cannot see a check that was dropped)), through `trust::require_program`, the chain the
//! progenitor would have run: this boot replaces the hand-over, so without it a bench card would
//! enter unchecked bytes and its kernel would carry no trust root for the seal to find. The hashing
//! happens at set-up, outside every timed window.

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::cap::{Rights, address_space_cap, page_frame_cap, rendezvous_cap};
use crate::{println, sched};

// **Every function of `bench` lives in a child named for it**, and this file is the index (§282
// (when a file is split, the parent becomes an index)). Each child takes `use super::*` and is
// imported back whole, so siblings reach each other as they did when they shared this file and
// `bench::run` still resolves. Cut by lane `maintainer/last-three-over-2000` (2026-10-12 UTC),
// which brought this file under §266 (a Rust source file stays under 2,000 lines). E1 and E4 are
// built on aarch64 and riscv64 only, as every item in them was.
mod timing;
use timing::*;
mod sequence;
pub use sequence::*;
mod switch;
use switch::*;
mod ipc;
use ipc::*;
mod spawn_map;
use spawn_map::*;
mod cycle_probe;
use cycle_probe::*;
mod el0;
use el0::*;
mod compute;
use compute::*;
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
mod thread_scaling;
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
use thread_scaling::*;
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
mod app_displacement;
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
use app_displacement::*;
mod filesystem;
use filesystem::*;
mod smp_throughput;
use smp_throughput::*;
