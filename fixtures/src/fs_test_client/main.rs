//! **The FS-service client** (milestone 32 phase 2): opens a file through a granted directory
//! capability and proves the whole stack end to end.
//!
//! This is the program a milestone-31 shell will one day be. It holds an endpoint to the FS server
//! and nothing that names the server, the block server, or the disk. That endpoint IS its directory
//! capability: it can open names the server resolves under the bound directory, and it can open
//! nothing else, because there is no global namespace to reach into. It reads the `motd` the image
//! ships with, then writes a pattern to `scratch` and reads it back, and reports the `motd` head
//! plus a success sentinel. Any failed check traps, which fails the waiting test.
//!
//! # Capability contract (notes/fs-server.md, notes/abi.md §4)
//! - **slot 0**: the file-service endpoint, `WRITE` (this is the directory capability; the client
//!   `CALL`s here).
//! - **slot 1**: the report endpoint, `WRITE`.
//! - **`channel::FILE_VA`**: the base of the channel shared with the FS server (a name out, file bytes
//!   both ways). `filesystem_contract::fs::TRANSFER_PAGES` pages, all of them mapped by this client's wiring,
//!   because the throughput role asks for the whole of it in one request.
//!
//! # Layout
//! One module per role (the proof, the benchmarks, the attackers and the witnesses), the requests
//! they share in `channel`, and in `role` the role numbers with the `_start` that dispatches on
//! them. Split out of one file on 2026-10-12 (UTC); the module names are provisional.
//!
//! Name: ratified 2026-08-01 (calef, milestone 63), replacing `fsclient`. Refused `fsclient`
//! (squished) and `fs_client` (any program that wants files is an FS client, so that name belongs
//! to the real thing; it also predicts "a client of the FS service" rather than "the program the
//! kernel spawns to prove the FS contract holds").

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

mod bench;
mod channel;
mod crash;
mod directory_attacker;
mod extended_attributes;
mod file_attacker;
mod name_set_attributes;
mod proof;
mod role;
mod shared_frame;
mod throughput;
mod two_directories;

user_mode_runtime::panic_handler!();
