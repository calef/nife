//! **The filesystem wire protocols** (notes/fs-server.md): one crate, so the block server, the FS
//! server, the client and the kernel-side tests share one definition and cannot drift. The kernel
//! routes these words as any IPC and never reads an opcode, so adding one changes this crate and the
//! note, not the syscall surface.
//!
//! ```text
//!   disk ──virtio──►┌──────────────┐──blk IPC──►┌───────────┐──file IPC──► client
//!                   │ block server │            │ FS server │
//!                   └──────────────┘◄───────────└───────────┘◄──────────── (a granted
//!                        (owns the DMA            (owns RedoxFS +            directory cap)
//!                         confinement)             its own heap)
//! ```
//!
//! Nobody names anyone else (notes/ipc-naming.md): the FS server holds "an endpoint I read and
//! write blocks on", the client "an endpoint bound to one directory". Rewire them and neither side
//! can tell.
//!
//! Control is by message and bulk by shared page, per §10 (process model: capability-based,
//! microkernel). Each protocol is an endpoint `CALL` of two words, answered through the one-shot
//! Reply capability; bytes travel only in the region the two parties share per channel:
//! [`blk::TRANSFER_MAX`] bytes for blk IPC, [`fs::TRANSFER_MAX`] for file IPC, holding a name on
//! open or data on read and write.
//!
//! Every reply's first word is an [`i64`]: non-negative is a result, negative is the negated errno
//! ([`reply_err`], [`reply_errno`]). RedoxFS's error carries Linux errno numbers, and the FS server
//! maps it to the wire once, at its serve loop, and nowhere else.
//!
//! Name: ratified 2026-08-23 (calef, a kernel-dependency crate naming review), renamed from
//! `fs_proto`; `_protocol` per calef's 2026-09-05 ruling.

#![no_std]

/// One page, in bytes. Also one RedoxFS block (`redoxfs::BLOCK_SIZE`), so a block move is a page
/// move and one frame of the direct map holds exactly one block.
///
/// **This is the unit both channels are built from, and neither channel's whole size any more**:
/// a `blk` request moves up to [`blk::TRANSFER_BLOCKS`] of these (milestone 138 step 4) and a file
/// request moves up to [`fs::TRANSFER_PAGES`] (step 3). A file read or write larger than
/// [`fs::TRANSFER_MAX`], or a `Disk` call spanning more than [`blk::TRANSFER_MAX`], is still
/// chunked by the caller.
pub const PAGE: usize = 4096;

/// Where a request packs its opcode: bits 63:56 of the first `CALL` word, the same position
/// `line_editor::proto` uses, so the two contracts read alike.
pub const OPERATION_SHIFT: u32 = 56;

/// Build a request's first word from just an opcode (the block protocol's shape: the operand, a
/// block index, rides in the second word).
pub const fn req(operation: u64) -> u64 {
    operation << OPERATION_SHIFT
}

/// The opcode of any request word.
pub const fn operation(w0: u64) -> u64 {
    w0 >> OPERATION_SHIFT
}

/// Turn a redox/POSIX errno into a reply's first word: the negated number, so the client can invert
/// it. This is the ONE place the error convention is defined; the FS server calls it at its boundary.
pub const fn reply_err(errno: i32) -> i64 {
    -(errno as i64)
}

/// The errno behind an error reply, or `None` if the reply is a (non-negative) success. The inverse
/// of [`reply_err`], for a client turning a wire error back into an `ErrorKind`.
///
/// # Examples
///
/// One word, no flag bit: a length and an errno come out of the same register.
///
/// ```
/// use filesystem_protocol::{reply_err, reply_errno};
///
/// // A READ that moved 4096 bytes, and a READ at end of file. Both are results.
/// assert_eq!(reply_errno(4096), None);
/// assert_eq!(reply_errno(0), None);
///
/// // ENOENT, mapped once at the server's serve loop and nowhere else.
/// let wire = reply_err(2);
/// assert_eq!(wire, -2);
/// assert_eq!(reply_errno(wire), Some(2));
/// ```
pub const fn reply_errno(r0: i64) -> Option<i32> {
    if r0 < 0 { Some((-r0) as i32) } else { None }
}

/// **The block-IPC protocol** (FS server → block server). Three synchronous methods shaped exactly
/// like the RedoxFS `Disk` trait the FS server implements over it: read a block, write a block, ask
/// the size. The unit of transfer is one filesystem block ([`PAGE`] bytes) as [`blk::SIZE`] and
/// [`blk::FLUSH`] see it; [`blk::READ`] and [`blk::WRITE`] may carry up to [`blk::TRANSFER_BLOCKS`]
/// of them in one request (milestone 138 step 4). The block server splits whatever it moves into
/// the 512-byte virtio sectors the device actually transfers, as a single descriptor spanning the
/// whole request rather than one per block.
pub mod blk;

/// **The file-service protocol** (client → FS server). The contract is capability-shaped from birth
/// (notes/fs-server.md): the endpoint a client holds IS the directory capability, bound in the
/// server to one directory node, and every name in [`fs::OPEN`] is resolved relative to it. There is no
/// global namespace and no absolute path; a client with no such endpoint can open nothing, and the
/// refusal is "no such capability" (it holds no endpoint), never a permission check. An [`fs::OPEN`]
/// hands back a **handle**, a small integer the server issues and validates against this session's
/// table; a handle is likewise a capability, meaningless to forge because the server only honors the
/// ones it minted.
pub mod fs;

/// **What a directory capability carries** (milestone 47): the rights ladder, and the one
/// constructor that makes attenuation monotonic.
///
/// A directory capability was one authority until this module existed, which meant that handing a
/// program somewhere to write its logs also handed it the power to delete what was already there.
/// Milestone 47's answer is that a directory is **separable rights** (six at the keystone, a
/// seventh added by DECISIONS §112 for `touch -t`'s arbitrary-mtime authority), and the answer to "can a
/// child ever carry more than its parent" is *no, by construction*: [`dir::Rights::attenuate`] is a
/// bitwise AND with the parent, it is the only way to make a non-root [`dir::Rights`], and `a & b`
/// is a subset of `a` for every `b`. There is no code path that widens, so there is nothing to
/// forget.
///
/// # The three refusals, and why they are three
///
/// The errno a missing right answers is a design decision, not a detail, because it decides what the
/// holder *learns*:
///
/// - **A naming right** ([`dir::READ`]/[`dir::WRITE`] for [`fs::OPEN`], [`dir::DESCEND`] for
///   [`fs::OPENDIR`]) answers `ENOENT`. In this scope there is no such name, which is the same
///   sentence `fs_file_caretaker` says for the same reason: a holder must not be able to map what
///   it cannot reach.
/// - **A mutating right** ([`dir::CREATE`], [`dir::REMOVE`], [`dir::SETTIME`], and [`dir::WRITE`]
///   on a file handle) answers [`dir::EROFS`]. Through this capability that directory is
///   read-only. `EACCES` was
///   rejected here for the reason DECISIONS §27 rejected it for files: it implies a policy that
///   could have said yes, and there is no policy, only what the capability is.
/// - **[`dir::ENUMERATE`]** answers [`dir::EPERM`], and it is the one that cannot use either of the
///   other two. "No such name" makes no sense (you hold the directory), and an empty listing would
///   be a statement about the *directory* rather than about the capability, which is exactly the
///   silent degradation DECISIONS §42 forbids.
pub mod dir;

/// **The verb table: what each request word means, declared once instead of three times**
/// (milestone 61).
///
/// Three caretakers (`components/src/fs_file_caretaker.rs`, `fs_subtree_caretaker.rs`,
/// `fs_nameset_caretaker.rs`) proxy this contract over a narrowed namespace. With a hand-written
/// `match` per caretaker, **a new verb is silently absent from a caretaker**. A verb declares its
/// **argument shape** (does the word's length field count a name, a payload, or nothing?) and the
/// **rights the server will demand**, and the caretakers dispatch off that instead of off a match
/// arm somebody has to remember to add. Adding a verb to [`fs`] without a row here is a **compile
/// error** (see [`verb::TABLE`]'s const assertions), which is the whole deliverable: forgetting now
/// fails the build rather than producing a capability that is quietly missing.
///
/// # What it shares, and the thing it must never share
///
/// **It shares the dispatch and never the attenuation.** The three caretakers narrow by three
/// different means and stay three programs:
///
/// - `fs_subtree_caretaker` performs **no checks at all**. One [`fs::OPENDIR`] at startup, the
///   server intersects the rights and mints a restricted handle, and everything afterwards is
///   reached through that handle. This table gives it its shape lookup and nothing else; it consults
///   no policy and no filter, because a branch here is a branch that can be wrong.
/// - `fs_nameset_caretaker` filters names, on **every** verb whose operand is a directory name, and
///   [`verb::Operand::Name`] is what tells it which those are, so a new name-taking verb cannot
///   arrive **unfiltered**.
/// - `fs_file_caretaker` translates between two *protocols*, directory in and file out, so it needs
///   a per-verb answer and has one in [`verb::file_grant::POLICY`]. Its refusals keep their distinct
///   errnos deliberately: [`grant::ENOTDIR`] means the request does not mean anything through a file
///   capability, which is a different statement from a refusal, and flattening the two into
///   "allowed / not allowed" would destroy it.
///
/// # BUGS
///
/// - **A wrong row is wrong in three programs at once.** The mitigation is that it is pure data in a
///   host-testable crate, reachable by the tests below and by Kani, which a hand-written match in a
///   `no_std` binary is not.
/// - **It does not make the caretakers interchangeable**, and must not try to. Only the dispatch is
///   shared; what each attenuates to stays hand-written and stays in three separate address spaces.
pub mod verb;

/// **How [`fs::READDIR`] packs a directory listing into the shared page.**
///
/// One record per entry: a flags byte, a length byte, then that many bytes of name. Length-prefixed
/// rather than NUL-terminated because a name is bytes to this contract and a terminator inside one
/// would silently truncate it, and one byte because RedoxFS's own limit on a directory entry is well
/// under 255.
///
/// The reply's `r0` is how many bytes of the page the server filled, so a client iterates until it
/// has consumed exactly that many. A record is never split across replies: the server stops before
/// one that would not fit and the cursor picks up there, so "the page was full" and "the directory
/// ended" are told apart by `r0` rather than by guessing.
pub mod dirent;

/// **How [`fs::STATFS`] packs the volume's numbers into the shared page** (milestone 54).
///
/// Three little-endian `u64`s, in this order: the allocation unit in bytes, the volume's size in
/// those units, and how many of them are free. Nothing else, because nothing else changes a
/// caller's decision: a client asks this to find out whether what it is about to write will fit,
/// and macOS asks it to size a Time Machine sparsebundle against the volume it is landing on.
///
/// # The three choices worth arguing with
///
/// **A record in the page rather than a packed reply word.** A reply carries one `i64`, and two
/// 64-bit counts do not fit in one however they are packed. Every other verb that answers with more
/// than a number ([`fs::READDIR`], [`fs::LISTXATTR`]) fills the page and returns its length, so this
/// is the contract's existing shape rather than a new one.
///
/// **The unit is a field, not [`PAGE`].** They are the same number today (4096) and they are not the
/// same *thing*: one is the filesystem's allocation granularity and the other is the IPC transfer
/// unit. A client that assumed they were equal would silently mis-size a volume the day a
/// filesystem with 64 KiB records is served through the same contract.
///
/// **The reply's length is the version.** `r0` is [`statfs::LEN`] today; a future field extends the
/// record and raises it, and a client written against this version reads its prefix correctly
/// because it checks `r0 >= LEN` for the fields it knows rather than `r0 == LEN`.
/// [`statfs::decode`] is written that way on purpose. There is deliberately **no version word**:
/// the length already is one, and a second one would be a thing to keep in sync.
///
/// # BUGS
///
/// - **Free space is a forecast, not a reservation.** See [`fs::STATFS`]'s own BUGS: the store's
///   `ENOSPC` at write time is the authority.
/// - **There is no inode count.** POSIX `statfs` carries `f_files`/`f_ffree`, and SMB's volume
///   classes carry neither, so nothing above this contract has asked. Adding them extends the
///   record by the paragraph above rather than changing it.
pub mod statfs;

/// **The per-file grant** (milestone 31 phase 2): what it means to hold one file rather than a
/// directory, and how the narrowing travels.
///
/// A directory capability lets its holder name anything in the bound directory. `run wc report.txt`
/// must hand over less than that: **one file, in one direction, and nothing else**. The narrowing
/// is done by an attenuator, `components/src/fs_file_caretaker.rs`, which holds the directory capability,
/// opens exactly the granted name once at startup, and then serves the *same* [`fs`] protocol on
/// its own endpoint with three rules:
///
/// 1. **[`fs::OPEN`] answers only the granted name.** Any other name is `ENOENT`, because in this
///    scope there is no such name; nothing consulted a permission. The holder cannot enumerate, and
///    cannot discover what else exists.
/// 2. **[`fs::CREATE`] is `ENOTDIR`.** A file capability is not a directory, so "create a name in it"
///    is not a request that means anything, which is a better answer than a permission refusal.
/// 3. **[`fs::WRITE`] and [`fs::TRUNCATE`] are [`grant::EROFS`] without [`grant::WRITE`].** The
///    capability is read-only; there is no policy to consult and no way to widen it from inside.
///
/// The attenuator pattern is Mark Miller's caretaker, and putting it in a separate process is what
/// makes the claim checkable: the confined program holds an endpoint to the caretaker and nothing
/// that names the FS server, so it cannot route around the narrowing even in principle.
/// Open-by-path still exists only inside a server (DECISIONS §27); the caretaker just serves a
/// server whose entire namespace is one name.
pub mod grant;

/// **A directory capability attenuated to a set of names** (milestone 47's globbing lane).
///
/// [`grant`] narrows a directory down to one name, which is what `wc report.txt` and `rm old.txt`
/// designate. `rm *.txt` designates more than one and fewer than all, and the roadmap's decided
/// answer is a directory capability attenuated to **the names that matched**. `fs_file_caretaker`
/// already serves a namespace of exactly one name; this is the same shape with a wider namespace,
/// served by `components/src/fs_nameset_caretaker.rs`.
///
/// # Why the set rides in a frame and the single name does not
///
/// One name fits in two `START` argument words, which is why a per-file grant costs no page. A set
/// does not fit in any number of registers, so it is written into a frame the wiring maps
/// **read-only** into the caretaker. That is the honest place for `ARG_MAX` to reappear: it is not
/// a buffer limit any more, it is the size of a capability, and [`nameset::MAX_NAMES`] is where the
/// ceiling is declared.
///
/// # The encoding, and why each name carries its type
///
/// A record is one header byte then the name: bit 7 of the header is [`nameset::IS_DIR`] and the low seven
/// are the length (a name is at most [`grant::MAX_NAME`] bytes, so seven bits is room to spare). A
/// zero header terminates the set, which is why a name may not be empty.
///
/// The type bit is carried because the caretaker **answers `READDIR` from the set itself** rather
/// than filtering the server's listing: the set is the namespace, so listing it needs no round trip
/// and no cursor translation. The type is what the directory said at the moment the grant was
/// planned, which is the same "resolved at grant time" rule every other part of a grant follows: a
/// `cd` after the fact cannot change what an already-planned grant means, and neither can a
/// `mkdir`.
pub mod nameset;

/// **Extended attributes: the limits, the encodings, and the store the FS server keeps them in**
/// (milestone 57, DECISIONS §34's 2026-07-31 amendment).
///
/// An extended attribute is a named byte string attached to a file or a directory. Samba stores
/// Apple's Time Machine metadata in them (`streams_xattr`), which is why they exist here at all, and
/// milestone 55's backup target is the deliverable on the other end.
///
/// # It is a layer, and the contract is why that is safe
///
/// RedoxFS has none of this, so it is built **above** the engine rather than forking the on-disk
/// format: this module is the contract, so where an attribute lives is invisible to every client.
///
/// On Linux a layer like this would be worthless, because anything can open the file directly and
/// walk around it. **Here nothing can.** Every path to the disk goes through [`fs`], so a layer above
/// the filesystem is as authoritative as the filesystem.
///
/// # There is no attribute right, and that is deliberate
///
/// [`fs::GETXATTR`] and [`fs::LISTXATTR`] need [`dir::READ`]; [`fs::SETXATTR`] and
/// [`fs::REMOVEXATTR`] need [`dir::WRITE`]. No seventh rung was added to the ladder. An attribute is
/// part of what a file *is*, not a separate object with its own authority, so a capability that may
/// read a file may read what is attached to it and a capability that may not write it may not
/// change them. A new rung would silently change every existing grant.
///
/// # The type code exists so indexing is not foreclosed
///
/// Every attribute carries a `u32` **kind**, and this layer never interprets it: it is stored,
/// returned, and otherwise ignored. [`xattr::RAW`] is what a POSIX-style client writes, and it is what
/// Samba will write.
///
/// The reason to carry a field nothing reads is `design/haiku-bfs-and-packages.md`: BFS made
/// attributes **typed and indexed**, and a store of untyped blobs cannot be indexed later without a
/// format migration and a wire break. The cost is four bytes per record and one packed word.
///
/// **BUGS.** The kind is 31 bits, not 32, because it rides in the sign-protected half of a reply word
/// (see [`xattr::reply`]). BFS-style four-character codes are ASCII and fit; a code with its top bit set does
/// not, and [`fs::SETXATTR`] answers `EINVAL` rather than truncating it.
pub mod xattr;

/// The FS server's readiness sentinel: sent once, after it has opened the RedoxFS image over blk
/// IPC and before it serves clients. The test waits for it, so a hang in `open` (the blk path)
/// is distinguishable from a hang in the serve/client path, and a booted-but-empty run is caught.
///
/// This word stayed in the protocol crate when the test fixture moved out to
/// `filesystem_fixture` (milestone 886, calef's ruling): it is a word shipping programs send
/// each other, and every other protocol here keeps its readiness word in its own crate
/// (`entropy_protocol`, `byte_sink_protocol`, `login_protocol`).
pub const READY: u64 = 0xF5_0BEEF5;

/// **A caretaker's descent was refused**, sent on the same endpoint [`READY`] would have gone
/// out on, with the errno in the second word. The caretaker then exits without serving.
///
/// It exists because of who waits for the handshake. Under `kernel::user::fs_service` the waiter
/// is a test, and a caretaker that panicked instead of answering cost that test a 60-second
/// watchdog and a legible failure. Under `system_initializer` the waiter is **init**, which is
/// the spawn service for the whole prompt and has no other thread: a caretaker that panicked
/// there would park init in `RECEIVE` forever and the machine would never take another command.
/// A grant naming a directory that is not there is an ordinary thing to type, so the honest
/// answer has to be a message rather than a corpse.
///
/// Name: provisional (2026-08-17, milestone 31 (a capability shell) phase 3): it names the one
/// step that can fail before a caretaker serves anything, which is the single `OPENDIR` its
/// whole attenuation lives in. It moved here from the fixture with [`READY`] (milestone 886).
pub const DESCENT_REFUSED: u64 = 0xF5_0DEAD5;

/// What the `rm` program reports, and how a diagnostic is told from the verdict.
pub mod rm;

#[cfg(test)]
mod tests;
