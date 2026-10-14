/// **The session's bound directory, which is always handle 0.**
///
/// The server installs its bound directory in the handle table before it serves anything, so the
/// directory a client's endpoint designates is an ordinary directory handle like any other and
/// every name-taking verb resolves against a handle rather than against a hidden field. That is
/// Plan 9's answer in one number: `/` is the root of *your* namespace, and two clients on two
/// endpoints both say `0` and mean different directories.
///
/// It is 0 because every client already sent 0 in the handle field of an [`OPEN`], which the
/// server ignored. Making 0 mean exactly what those clients meant costs no wire change; file
/// handles now start at 1.
pub const ROOT: u64 = 0;

/// Resolve the name in the shared page (its length is [`req_len`] of the request word) under the
/// directory handle in [`req_handle`] ([`ROOT`] for the endpoint's bound directory). Reply `r0`
/// = a handle (≥ 0) or an error, and `r1` = the file's size in bytes (see below).
///
/// Needs [`super::dir::READ`] or [`super::dir::WRITE`] on that directory, and the file handle it
/// returns **inherits both bits** from it, so what you may do to the file was decided when the
/// directory was granted. Without either the answer is `ENOENT`: in this scope there is no such
/// name. `EISDIR` if the name is a directory ([`OPENDIR`] is the verb for that).
///
/// **The name may be a relative path** (milestone 606 (a directory walk costs what it does on
/// Linux), calef's ruling A on #1387, 2026-09-27). `a/b/f` means exactly the hop-by-hop walk a
/// client would otherwise send: [`OPENDIR`] `a` asking for `DESCEND | w1`, [`OPENDIR`] `b` asking
/// for the same, then `OPEN f` in `b`, with every intermediate handle closed. So each directory
/// on the way needs `DESCEND` on its parent and is narrowed to `DESCEND | w1`, a step that comes
/// up short is [`super::dir::EPERM`], and the file handle carries what the walk would have given
/// it. The **second word `w1`** is that per-step request (a [`super::dir`] mask, in practice
/// `READ`, `WRITE` or both). A single-component name ignores it, as `OPEN` always did, and every
/// component is still one name: an empty step, `.` or `..` is `EINVAL`. A server that predates
/// this answers any name with a `/` in it with `EINVAL`, which is how a client knows to fall back
/// to the hop-by-hop walk.
///
/// **The reply's second word is the file's size** (ruling B, same day), so a client reading a
/// whole file can size its buffer without an [`FSTAT`]. It is a hint for exactly that and nothing
/// else: it is the size at the moment of the open, and a client must not answer a later
/// "what is the size" from it, because the file may have changed since. A caretaker or server
/// that does not fill it replies 0, so 0 means "not given" and a client asks with [`FSTAT`]; for
/// an empty file that costs one message.
pub const OPEN: u64 = 1;
/// Read up to [`req_len`] bytes from the handle ([`req_handle`]) at offset `w1` into the shared
/// page. Reply `r0` = bytes read (≥ 0, 0 at EOF) into the shared page, or an error.
pub const READ: u64 = 2;
/// Write [`req_len`] bytes from the shared page to the handle at offset `w1`. Reply `r0` = bytes
/// written (≥ 0), or an error.
pub const WRITE: u64 = 3;
/// Close the handle ([`req_handle`]). Reply `r0` = 0, or an error if the handle was not open.
pub const CLOSE: u64 = 4;
/// The current size in bytes of the handle's file. Reply `r0` = size (≥ 0), or an error.
pub const FSTAT: u64 = 5;
/// Create the name in the shared page (length is [`req_len`]) under the directory handle in
/// [`req_handle`] ([`ROOT`] for the endpoint's bound directory) and open it. Second word 0.
/// Reply `r0` = a handle (≥ 0), or an error.
///
/// Needs [`super::dir::CREATE`] on that directory; without it the answer is
/// [`super::dir::EROFS`], because through this capability the directory takes no new names.
///
/// **`EEXIST` if the name already exists, and nothing is modified.** Create is create, not
/// create-or-open: a caller that wants either must ask for both and say which it got. The
/// alternative makes a partly-working write read as a working one, which is the failure §27
/// records. Shares [`OPEN`]'s shape exactly, so a client that can open can create.
pub const CREATE: u64 = 6;
/// Set the size of the handle's ([`req_handle`]) file to exactly `w1` bytes. [`req_len`] is 0;
/// the size rides in the second word because it is an offset-shaped quantity, not a payload
/// length. Reply `r0` = 0, or an error.
///
/// POSIX `ftruncate` in **both** directions: shrinking discards the bytes past the new size,
/// growing extends with zeroes. The shrink is the point. Without it a write shorter than the file
/// leaves the old tail in place, so a caller replacing a file's contents gets a longer file than
/// it wrote, and a write that half-works reads as a write that failed (DECISIONS §27, four times
/// corrected). Truncating to the current size is a no-op, which matters because `std::fs::write`
/// truncates unconditionally.
pub const TRUNCATE: u64 = 7;
/// **Descend: resolve the name in the shared page under the directory handle in [`req_handle`]
/// and hand back a handle to the child directory, carrying rights.** The requested rights ride
/// in the second word as a [`super::dir`] mask. Reply `r0` = the new directory handle (≥ 0), or
/// an error.
///
/// This is the first verb in this contract that hands back **authority** rather than data, which
/// is why its rules are stated here rather than left to the implementation:
///
/// - The child's rights are `parent & requested`, computed by
///   [`super::dir::Rights::attenuate`], which is the only constructor for a non-root rights set
///   and cannot widen. A child of a child is attenuated again, so **no descendant of a directory
///   capability can carry a right the capability did not have**, at any depth.
/// - If that intersection is not what was asked for, the request is refused with
///   [`super::dir::EPERM`] rather than quietly granted the smaller set. The refusal is a
///   courtesy, not the safety property: delete it and the intersection above still holds.
/// - It needs [`super::dir::DESCEND`] on the parent. Without it the answer is `ENOENT`, so a
///   holder that may not walk into a subtree cannot learn that the subtree is there.
/// - `ENOTDIR` if the name is a file. `EINVAL` if any component is empty, `.` or `..`, which is
///   what keeps `..` from meaning anything.
/// - **The name may be a relative path** (milestone 606 (a directory walk costs what it does on
///   Linux), calef's ruling A on #1387, 2026-09-27), resolved as the hop-by-hop walk would be:
///   every directory before the last is a descent asking for `DESCEND | requested`, and the last
///   asks for `requested`. Each step obeys the three rules above, so a path reaches nothing the
///   walk could not. A server that predates this answers `EINVAL`.
pub const OPENDIR: u64 = 8;
/// **Enumerate a directory handle.** [`req_handle`] is the directory, the second word is a
/// **cursor**: the index of the first entry to return, 0 to start. The reply's `r0` is the
/// number of bytes written into the shared page, encoded as [`super::dirent`] records; `r0` = 0
/// means the cursor is past the end. [`req_len`] is ignored.
///
/// Needs [`super::dir::ENUMERATE`], and its absence is [`super::dir::EPERM`], **not an empty
/// listing**. An empty listing would be a lie about the directory rather than a fact about the
/// capability, and DECISIONS §42's rule is that a verb which is not offered fails loudly instead
/// of degrading silently.
///
/// Entries come back sorted by name so the cursor means the same thing across calls; a directory
/// changed between two calls of one enumeration can therefore repeat or skip a name, which is
/// the ordinary readdir caveat and is recorded rather than fixed.
pub const READDIR: u64 = 9;
/// **Make a child directory and hand back a capability to it**, which is why it lives here
/// rather than beside [`CREATE`]: `mkdir` is descend-with-creation, and milestone 47's
/// instruction is that the two be designed together rather than separately.
///
/// Shares [`OPENDIR`]'s shape exactly (the name in the shared page, the requested rights in the
/// second word, a directory handle in the reply) and differs in two things: it needs
/// [`super::dir::CREATE`] as well as [`super::dir::DESCEND`], and it answers `EEXIST` if the
/// name is already there rather than opening what it found. Create is create, for the reason
/// [`CREATE`] gives.
///
/// The new directory's rights are attenuated from the parent's exactly as [`OPENDIR`]'s are, so
/// a program cannot mint itself more authority by *making* a directory than by finding one.
pub const MKDIR: u64 = 10;
/// **Move a name from one directory to another, or to a different name in the same one.**
///
/// The only verb in this contract that names **two** directories, so it is the only one whose
/// second word is not a scalar: the source is [`req_handle`]/[`req_len`] of the request word as
/// usual, and the destination rides in the second word packed by [`rename_dst`], which uses the
/// same two fields. Both names are in the shared page, the **source first**, back to back, so
/// the source occupies `[0, req_len)` and the destination `[req_len, req_len + dst_len)`.
/// Reply `r0` = 0, or an error.
///
/// # The two atomicities, stated apart (DECISIONS §42)
///
/// - **Concurrency-atomic: yes.** The FS server runs one request to completion before it
///   receives the next, so no observer sees the name in both places or neither.
/// - **Crash-atomic: yes.** The whole rename runs in one RedoxFS transaction and reaches the
///   platter through one commit in the header ring, so a fresh mount finds the old name or the
///   new one and never both or neither; `redoxfs_server/tests/crash_consistency.rs` cuts the
///   device at every write of a workload that ends with a rename.
///
/// The temp-file-then-rename idiom needs both, which is why §42 states them separately.
///
/// # Rights, and what is deliberately not offered
///
/// Needs [`super::dir::REMOVE`] on the **source** directory (its name goes away) and
/// [`super::dir::CREATE`] on the **destination** (a name appears). Both are mutating rights, so
/// a capability lacking either answers [`super::dir::EROFS`]: through this capability that
/// directory does not give names up, or does not take new ones. Within one directory a rename
/// needs both on that one directory.
///
/// - **`renameat2`'s `EXCHANGE` and `NOREPLACE` are not offered** (§42): they work on a few
///   Linux filesystems only, so offering them would make behavior backend-specific.
/// - **Cross-filesystem move is not this verb** (§42): both handles are minted by one server
///   bound to one image.
/// - **A directory can be renamed in place but not moved into another directory** (`EINVAL`):
///   POSIX's guard against making a directory its own descendant is a path-prefix test, and the
///   equivalent here is an ancestry walk in a server whose stack is three quarters used.
///
/// If the destination name exists it is **replaced**, provided it is the same kind as the
/// source: a file over a directory is `EISDIR` and a directory over a file is `ENOTDIR`, POSIX's
/// own answers, and a non-empty destination directory is `ENOTEMPTY`, checked here so the verb
/// means one thing on every backend. Renaming a name onto itself is a successful no-op.
pub const RENAME: u64 = 11;
/// **Remove a name from a directory. This is `unlink`, and it is not revocation.**
///
/// The name in the shared page (length is [`req_len`]) is taken out of the directory in
/// [`req_handle`]. Second word 0. Reply `r0` = 0, or an error.
///
/// # The two operations Unix conflated, and which one this is
///
/// Milestone 47's instruction is that `rm` stop meaning both at once, so this verb means exactly
/// one of them and says which:
///
/// - **Unlink (this verb).** The *name* goes away. A handle already open on that file **keeps
///   reading**, and the bytes are still there, exactly as POSIX promises; the object dies when
///   the last name and the last handle are both gone. Atomic replace and the temp-file idiom
///   both depend on this, which is why it is worth having rather than an accident to be tidied
///   up. The FS server makes it true by registering an open node with the engine's usage table,
///   so removing the last link **queues** the node rather than freeing it (`release_node`).
/// - **Revoke.** The object dies and every capability to it goes stale. Not offered here, and
///   the absence is deliberate rather than an oversight: it would mean invalidating handles the
///   server minted for clients it cannot enumerate (the handle table is per *server*, and the
///   server does not track which client holds which handle). §13 revokes frames and §16 revokes
///   objects because the kernel owns those tables; nothing owns this one that way yet. A verb
///   that removed a name and *called itself* revocation would be the silent degradation §42
///   forbids, so `rm` unlinks and says so.
///
/// # Rights, and the kind check
///
/// Needs [`super::dir::REMOVE`] on the directory, refused with [`super::dir::EROFS`]: through
/// this capability that directory does not give names up. The right is checked **before the name
/// is resolved**, so a capability that may not remove cannot use this verb to find out whether a
/// name is there.
///
/// **A directory is refused with [`super::dir::EISDIR`]**, POSIX's own answer. [`RMDIR`] is the
/// verb for a directory, and it takes only an **empty** one, so no single call on this contract
/// can take a subtree away. "Remove this name whatever it is" is how `rm` destroys a tree for
/// someone who typed one word, and this contract does not offer it at any opcode.
pub const UNLINK: u64 = 12;
/// **Remove an empty directory. Unix's `rmdir(2)`, and empty-only is the whole safety
/// property.**
///
/// [`UNLINK`]'s shape exactly: the name in the shared page (length is [`req_len`]), resolved
/// under the directory handle in [`req_handle`], second word 0, reply `r0` = 0 or an error. It
/// hands nothing back, which is what separates removing a name from destroying an object.
///
/// # Empty-only, and why that is not a limitation to be lifted
///
/// A directory with anything in it is [`super::dir::ENOTEMPTY`], refused rather than emptied.
/// **The recursion in `rm -r` lives in userspace** (`components/src/rm.rs`), as a loop of individually
/// safe single-step operations: enumerate, unlink the files, remove the empty directories from
/// the bottom up. Each step is one request this server runs to completion, and each step needs
/// the rights for it at *that* level, so a recursive removal stops exactly where the
/// capabilities stop. A `RMDIR` that removed whatever it found would put the whole subtree
/// behind one call, and no capability check afterwards could undo that.
///
/// This also means an interrupted `rm -r` leaves a **partial tree**, which is what Unix does
/// too. There is no transaction spanning requests, and adding one would mean the server holding
/// a transaction open across receives, which breaks the run-one-request-to-completion property
/// the concurrency atomicity of [`RENAME`] rests on.
///
/// # Rights, the kind check, and what this is not
///
/// Needs [`super::dir::REMOVE`] on the parent, refused with [`super::dir::EROFS`] and checked
/// **before the name is resolved**, so a capability that may not remove cannot use this verb to
/// find out whether a name is there. A **file** is `ENOTDIR`, POSIX's own answer for `rmdir` on
/// one, and the mirror of [`UNLINK`]'s `EISDIR`: neither verb will remove the kind the other one
/// is for.
///
/// **It is not revocation**, for [`UNLINK`]'s reason and not a lesser version of it: the FS
/// server's handle table is per *server*, so handles cannot be invalidated for clients the
/// server cannot enumerate. A holder of a directory handle whose name this verb removes keeps a
/// handle onto a node the engine has freed, and its next request through it fails (`ENOENT`)
/// rather than reaching something else. See the BUGS section of notes/rm.md.
pub const RMDIR: u64 = 13;

/// **Read one extended attribute of the node a handle names** (milestone 57).
///
/// The attribute's name is [`req_len`] bytes at the start of the shared page, the handle in
/// [`req_handle`] is the **file or directory** the attribute is on, and the second word is 0.
/// Reply `r0` = [`super::xattr::reply`], which packs the value's type code and its length; the
/// value itself lands in the shared page. `ENODATA` if the attribute is not set.
///
/// Needs [`super::dir::READ`] on the handle, refused with `EBADF`, which is exactly what
/// [`READ`] needs and answers: an attribute is part of what a file *is*, so the direction the
/// capability carries is the direction that governs it. See [`super::xattr`] for why there is no
/// separate attribute right.
pub const GETXATTR: u64 = 14;
/// **Set one extended attribute on the node a handle names** (milestone 57).
///
/// The only verb here that carries two payloads and a type: the name is [`req_len`] bytes at the
/// start of the shared page and the value follows it back to back, with the value's length and
/// its type code packed into the second word by [`super::xattr::spec`]. Reply `r0` = 0.
///
/// Creating and replacing are one operation, unlike [`CREATE`]: an attribute has no handle and
/// no identity of its own, so "it already had a value" is not a fact a caller can act on
/// differently. Linux's `XATTR_CREATE`/`XATTR_REPLACE` flags are deliberately not offered, for
/// DECISIONS §42's reason: they are emulable only racily above the store.
///
/// Needs [`super::dir::WRITE`], refused with [`super::dir::EROFS`]. `ERANGE` for a name this
/// store cannot hold, [`super::xattr::E2BIG`] for an over-long value, and
/// [`super::xattr::ENOSPC`] when the node already holds [`super::xattr::MAX_COUNT`] attributes.
pub const SETXATTR: u64 = 15;
/// **List the attribute names on the node a handle names** (milestone 57).
///
/// [`req_len`] and the second word are both 0. Reply `r0` = how many bytes of the shared page
/// were filled with [`super::xattr::list`] records; 0 means the node has no attributes.
///
/// There is **no cursor**, unlike [`READDIR`], and that is a property rather than an omission:
/// [`super::xattr::MAX_COUNT`] names of [`super::xattr::MAX_NAME`] bytes fit one page exactly,
/// so a listing is always complete in one reply and cannot be observed half-changed.
///
/// Needs [`super::dir::READ`], refused with `EBADF`, for [`GETXATTR`]'s reason.
pub const LISTXATTR: u64 = 16;
/// **Remove one extended attribute from the node a handle names** (milestone 57).
///
/// [`GETXATTR`]'s shape: the name is [`req_len`] bytes at the start of the shared page, the
/// second word is 0. Reply `r0` = 0, or `ENODATA` if the attribute was not set. Removing an
/// attribute that is not there is an **error, not a no-op**, because the caller asked about a
/// specific thing and "it was not there" is the answer.
///
/// Needs [`super::dir::WRITE`], refused with [`super::dir::EROFS`].
pub const REMOVEXATTR: u64 = 17;

/// **How much room the store behind this capability has** (milestone 54). POSIX's `statfs`, cut
/// down to the three numbers a client actually decides with.
///
/// [`req_len`] and the second word are both 0; [`req_handle`] is **any handle this server
/// minted**, file or directory, [`ROOT`] included. Reply `r0` = how many bytes of the shared
/// page were filled, which is [`super::statfs::LEN`], or an error; the record itself is
/// [`super::statfs`]. `EBADF` for a handle the server did not mint.
///
/// # Why any handle, and why no right
///
/// The answer is a fact about the **storage behind the capability**, not about anything the
/// capability names, and every handle in one server is on one image. So this takes a handle the
/// way [`FSTAT`] and [`CLOSE`] do, for the same reason: holding one is the whole
/// qualification, and a caller with no handle asks nothing. Demanding [`super::dir::READ`]
/// would have made a write-only grant unable to find out whether its next write fits, which is
/// the one question it has.
///
/// The handle is still checked, and that is the capability part: a client that holds no
/// endpoint into this server cannot ask, and one whose endpoint is a caretaker asks through the
/// caretaker's own table. There is no ambient "how full is the disk" call.
///
/// # BUGS
///
/// - **It answers about the whole image, never about a subtree.** A holder of a narrow subtree
///   capability learns the volume's free space, which is more than its own namespace tells it.
///   There are no quotas in this filesystem, so there is no smaller number that would be true;
///   reporting the subtree's own usage as a "size" would be the silent degradation DECISIONS
///   §42 forbids. If quotas ever exist, this verb reports the quota and this note goes away.
/// - **The number moves when somebody else writes, so it is a channel and not only a leak**
///   (recorded-accepted). Two programs confined to disjoint subtrees of one image can communicate:
///   one writes and truncates to modulate the free count, the other polls `STATFS`. The bandwidth
///   is unmeasured. It is not fixable at this verb, since a true answer moves when the volume
///   moves; per-subtree quotas would replace it with a private number. Until then **two subtrees
///   on one image are not isolated from each other's write volume**; isolation needs two images.
/// - **The numbers are a snapshot with no lock behind it.** Two clients writing concurrently
///   both see a free count that was true when it was read; the store's own `ENOSPC` at write
///   time is the authority, and this is a forecast. That is what `statfs` is everywhere.
pub const STATFS: u64 = 18;

/// **Make everything this server has acknowledged durable** (milestone 55 (Time Machine)).
///
/// [`req_len`] and the second word are both 0; [`req_handle`] is **any handle this server
/// minted**, file or directory, [`ROOT`] included, as [`STATFS`] takes one. Reply `r0` = how many
/// device flushes the storage behind this capability has completed since the block server
/// started, **including this one**, so a success is always `>= 1`; or an error. `EBADF` for a
/// handle the server did not mint.
///
/// It promises that the **device** has been told and the reply waited for it to finish. The FS
/// server commits every write in one RedoxFS transaction before replying, so there is no dirty
/// state above the block device. **After a successful `SYNC`, every write this server has
/// acknowledged is on the medium the device calls durable**; a device that lies about its flush is
/// outside what a protocol can check.
///
/// Needs [`super::dir::WRITE`], refused with [`super::dir::EROFS`]: a sync is an action on the
/// device (a flush stalls the queue for every client of the image), and a holder that may not
/// write has nothing outstanding to make durable.
///
/// When the device cannot flush, the block server's `EOPNOTSUPP` (95) passes **through**
/// unchanged, the only errno on this contract that does not originate in the FS server or
/// RedoxFS. Answering 0 would be the silent degradation that DECISIONS §42 (a filesystem declares
/// what it offers and must be truthful) forbids, where the consequence is somebody's backup.
///
/// # BUGS
///
/// - **The count is a fact about the device, not about the caller**, so two clients of one image
///   can watch each other sync: [`STATFS`]'s channel again, at lower bandwidth.
/// - **It syncs the whole device, never a subtree or a file**, because RedoxFS commits across the
///   image. Hence `SYNC` rather than `FSYNC`.
/// - **It does not fence.** A write and a sync issued concurrently on two handles are unordered;
///   the sync covers what the server had already acknowledged when it ran.
pub const SYNC: u64 = 19;

/// **The modification time of `name`, in Unix seconds** (milestone 47's `touch`, the read
/// half; DECISIONS §112). Resolved directly under the directory handle in [`req_handle`],
/// [`UNLINK`]'s shape exactly: the name is [`req_len`] bytes at the start of the shared page,
/// second word 0. Reply `r0` = the mtime (≥ 0), or an error.
///
/// **Name-taking rather than handle-taking, on purpose.** Every other fact this contract
/// answers about a file ([`FSTAT`], [`GETXATTR`]) needs an open handle first. This one does
/// not, because [`SETMTIME`] and [`SETMTIME_AT`] do not either: `touch` never opens what it
/// acts on (`components/src/swish.rs`'s `touch` builtin resolves a name straight under the
/// directory capability it already holds, the same shape [`CREATE`] and [`UNLINK`] use), and a
/// getter that needed a handle when the setters do not would be an asymmetry nothing forces.
///
/// Needs [`super::dir::READ`], and its absence answers `ENOENT` rather than [`super::dir::EROFS`]
/// or [`super::dir::EPERM`]: this is a **naming right**, [`OPEN`]'s own rule (see this module's
/// header): a holder that may not reach a name must not learn a fact about it, timestamp
/// included. Checked before the name is resolved, [`UNLINK`]'s reason: a holder that may not
/// read a time must not use this verb to find out whether a name is there.
///
/// Works on a directory exactly as on a file, [`GETXATTR`]'s reason: mtime is part of what a
/// node *is*, not a property files alone carry.
///
/// Name: provisional (milestone 47 (navigation and naming)'s mtime lane, 2026-08-24); an architect's
/// to ratify.
pub const GETMTIME: u64 = 20;

/// **Set `name`'s modification time to now** (milestone 47's bare `touch`; DECISIONS §112).
/// [`GETMTIME`]'s shape: the name is [`req_len`] bytes at the start of the shared page,
/// resolved under [`req_handle`], second word 0 (there is nothing for the caller to supply;
/// "now" is not the caller's to assert). Reply `r0` = 0, or an error.
///
/// # What "now" is, honestly
///
/// This server has no RTC wired to it (the same gap `redoxfs_server::Server`'s `clock` field
/// records): "now" is this server's own advancing logical clock, the exact mechanism that
/// already timestamps
/// every [`WRITE`], [`CREATE`], [`TRUNCATE`], [`SETXATTR`] and [`REMOVEXATTR`], not a reading
/// of the wall clock milestone 51 landed (`clock_protocol`, DECISIONS §43). A `touch` on two
/// files one after another is guaranteed to observe the second mtime greater than the first,
/// which is what every caller of `touch` actually depends on (make-style staleness checks
/// included); it is not guaranteed to match a wall-clock reading taken at the same instant.
/// Wiring this server to a real wall-clock mapping is a follow-up (see this crate's module
/// docs and notes/touch.md's `BUGS`), not a difference in what this verb promises today.
///
/// # Rights, and why this needs no more than [`WRITE`]
///
/// Needs [`super::dir::WRITE`], refused with [`super::dir::EROFS`], checked before the name is
/// resolved for [`UNLINK`]'s reason. **This is the half of `touch` that is not an assertion**:
/// the value written is one the server observed, bounded by what it already knew, the same
/// argument DECISIONS §112 makes from POSIX's own `utime()` split and from §43's read/set
/// asymmetry on the wall clock itself. A capability that may write a file may therefore also
/// record that it was written, without a separate grant.
///
/// Name: provisional (milestone 47's mtime lane, 2026-08-24); an architect's to ratify.
pub const SETMTIME: u64 = 21;

/// **Set `name`'s modification time to a value the caller supplies** (`touch -t`; DECISIONS
/// §112: the ability to *lie about history*). [`GETMTIME`]'s shape, with one addition: the
/// second word carries the Unix-seconds value to write, [`TRUNCATE`]'s reason for putting an
/// offset-shaped quantity in `w1` rather than in the length field. Reply `r0` = 0, or an error.
///
/// # Rights: [`super::dir::WRITE`] **and** [`super::dir::SETTIME`]
///
/// [`SETMTIME`] needs only [`super::dir::WRITE`] because the value it writes is one the server
/// itself observed. This verb writes a value **the caller asserts**, nothing bounds it, and
/// that is exactly the authority DECISIONS §112 declines to fold into [`super::dir::WRITE`]:
/// "no, they are not the same right." Refused with [`super::dir::EROFS`] for either bit
/// missing, checked before the name is resolved for [`UNLINK`]'s reason. A capability that may
/// bump a file's mtime to now may not thereby assert an arbitrary one; the two are the same
/// asymmetry DECISIONS §43 already drew between reading the wall clock (broadly grantable) and
/// setting it (a distinct, more tightly held authority), one level down, from a file's clock to
/// the machine's.
///
/// No range check on the seconds value: any `u64` is accepted, including one that predates the
/// image or postdates the machine's own clock, because asserting an impossible history is the
/// same authority as asserting a merely surprising one and this contract does not referee which
/// lies are plausible.
///
/// Name: provisional (milestone 47's mtime lane, 2026-08-24); an architect's to ratify.
pub const SETMTIME_AT: u64 = 22;

/// **Bind a badge to a directory: a subtree grant the server enforces itself** (milestone 606
/// (a directory walk costs what it does on Linux), calef's ruling D, 2026-09-27). [`req_handle`]
/// is a directory handle the caller holds, the second word is a badge: in practice a client
/// window's index, under §230 (badged endpoint capabilities). From then on a request carrying that
/// badge sees the directory as its [`ROOT`] and reaches only the handles it minted, through the
/// `subtree_scope` crate. Reply `r0` = 0 or an error.
///
/// The rules are `subtree_scope::Bindings::bind`'s. Only a caller whose own badge is unbound
/// may bind, because only such a caller already holds everything a binding could hand out.
/// Badge 0 cannot be bound, and a bound badge cannot be bound again. A refusal is `EPERM`.
///
/// **Not a directory verb**, and deliberately outside [`super::verb::TABLE`]'s range, so
/// `verb::of` knows nothing of it and no caretaker forwards it: a client behind a caretaker
/// could otherwise re-bind or revoke another client's badge. The binder is whoever holds an
/// unbound endpoint to the server, which is the progenitor, or the kernel's own test wiring.
///
/// Caretakers stay the default. A server takes this verb only if it is eligible under ruling D:
/// memory-safe, and resolving every path and handle through `subtree_scope`. `redoxfs_server`
/// is the first. One that is not answers `EINVAL`, as for any unknown opcode.
///
/// Name and number provisional; the number sits above the directory verbs on purpose.
pub const BIND: u64 = 64;
/// **Take a badge's grant back** ([`BIND`]'s inverse). The second word is the badge; the handle
/// field is ignored. The server closes the grant's root and every handle the badge minted, and
/// the badge reaches nothing from then on: it never falls back to the endpoint's whole
/// authority. Same caller rule as [`BIND`]. Reply `r0` = 0 or an error.
///
/// This is revocation for a server-enforced grant. The kernel has no way to take a capability
/// back out of a client's table, so the client keeps an endpoint that answers `EBADF`, where a
/// caretaker grant's revocation ended the caretaker instead.
pub const UNBIND: u64 = 65;

/// **Make a badge sync-only** (milestone 805 (`reboot` at the prompt), DECISIONS §251
/// (restarting the machine is a kernel object the progenitor hands out), its 2026-10-06
/// amendment). The second word is the badge; the handle field is ignored. From then on a request
/// carrying that badge may be [`SYNC`] and nothing else: the server answers `SYNC` with the
/// block server's flush count, as for any writer, and refuses every other verb with `EPERM`
/// before it reads a handle. The badge names no directory and reaches no file.
///
/// [`BIND`]'s caller rule (only an unbound badge may bind) and its revocation ([`UNBIND`] leaves
/// the badge reaching nothing). `SYNC` needs [`super::dir::WRITE`] on a handle for every other
/// client; the binder holds it, which is what lets it hand a badge the sync alone.
///
/// The one holder is `reboot`, which syncs for itself before it restarts the machine.
/// Outside [`super::verb::TABLE`]'s range for [`BIND`]'s reason: no caretaker forwards it.
///
/// Name: ratified 2026-10-06 (calef, #1783: "Approve sync for the file-server
/// request and its capability, and flush for the device cache only."). The number is provisional.
pub const BIND_SYNC: u64 = 66;

/// **How many pages the file channel spans** (milestone 138 step 3). The client and the FS
/// server share this many *contiguous* pages, not one, and a [`READ`] or [`WRITE`] may carry
/// up to [`TRANSFER_MAX`] bytes through them in a single request.
///
/// # Why this is the whole of the change
///
/// The request word has carried a 40-bit length field, [`MAX_LEN`], since this contract was
/// written, so nothing in the packing ever limited a transfer to 4096 bytes. **The page did.** Bulk rides in the
/// region the two parties share, that region was one page, and so a request that asked for
/// more would have had the server write past the end of it. Growing the region is therefore
/// the entire multi-page transfer: no new opcode, no second reply word, no descriptor, and no
/// change to a single packed field. Milestone 38's `BUGS` entry called the fix "a multi-page
/// transfer on the contract"; this is what that turned out to cost.
///
/// 16 pages, so [`TRANSFER_MAX`] is 64 KiB, which is the size milestone 38 measured ext4
/// moving "for about what it charges for 4 KiB". It is a tuning number and it is the one place
/// to change it.
///
/// # What a party has to map, and the one thing that is not checked
///
/// The FS server maps all [`TRANSFER_PAGES`] pages, because it must be able to serve any
/// request this contract permits. **A client maps as many as it intends to use**, and asks for
/// no more than it mapped. A client that only ever moves 4096 bytes therefore needs no change
/// at all: it maps one page, sends `len` 4096, and cannot tell that the region behind the
/// server grew. That is the compatibility property, and it is a real one rather than a
/// courtesy: `swish` and the caretakers are unmodified by this step.
///
/// **The foot gun, marked because it is rung four of `CLAUDE.md`'s ladder and not rung one:**
/// nothing checks that a client asked for no more than it mapped. A client that sends
/// `len = TRANSFER_MAX` with one page mapped gets a data abort on its own second page, and it
/// gets it *after* the server has already done the work. This is the same agreement a client
/// already has with its wiring about where the region lives at all (every FS client hardcodes
/// its own `FILE_VA` and the kernel's wiring carries a comment saying it must match), one
/// number wider rather than a new category. Making it unrepresentable needs the channel size
/// to travel on the wire, which is a new concept on this contract and is not this step's.
pub const TRANSFER_PAGES: usize = 16;

/// **The largest payload one [`READ`] or [`WRITE`] may carry**, in bytes: the whole file
/// channel ([`TRANSFER_PAGES`] pages). The FS server clamps every request's length to this,
/// which is the one bound that keeps a request inside the region it shares.
///
/// Setting [`TRANSFER_PAGES`] to 1 makes this [`super::PAGE`] again and reproduces the
/// contract exactly as it stood before milestone 138 step 3, which is what
/// `bench/transfer-size-sweep.sh` uses to measure a before against an after on one harness.
///
/// # Examples
///
/// A bulk request is the same word with a bigger length, and a one-page client is untouched.
///
/// ```
/// use filesystem_protocol::{PAGE, fs};
///
/// // The largest read this contract permits, in one request.
/// let w0 = fs::req(fs::READ, 3, fs::TRANSFER_MAX as u64);
/// assert_eq!(fs::req_len(w0), fs::TRANSFER_MAX);
/// assert_eq!(fs::TRANSFER_MAX, fs::TRANSFER_PAGES * PAGE);
///
/// // A client that shares only the first page keeps working untouched: it asks for one page,
/// // and one page is what the region behind its request has always been.
/// let w0 = fs::req(fs::READ, 3, PAGE as u64);
/// assert_eq!(fs::req_len(w0), PAGE);
/// ```
pub const TRANSFER_MAX: usize = TRANSFER_PAGES * super::PAGE;

/// **How many client channels one file service keeps side by side** (milestone 599 (a frame per
/// filesystem client channel), provisional). Each channel is its own [`TRANSFER_MAX`]-byte
/// staging window, backed by its own frame, so two clients no longer share one page: the file
/// server maps `CLIENT_WINDOWS` windows and picks window `badge` for each request
/// (`abi::rendezvous::BADGE`), and the wiring hands each client its own frame and a badged
/// endpoint naming its window. Window 0 is the unbadged default, so a single-client boot path is
/// unchanged.
///
/// **This bounds the number of file clients a boot can run at once**, which is the capacity
/// that used to be "however many the shared frame could be corrupted across". Eight is a
/// provisional value: it covers the heaviest path this tree wires today (a shell, a set-grant
/// caretaker, a `>` file caretaker, and headroom) and costs the server `CLIENT_WINDOWS *
/// TRANSFER_MAX` = 512 KiB of mapped frames. The production value is a capacity decision; a
/// lane ships this one and says so. See `notes/a-frame-per-filesystem-client-channel.md`.
pub const CLIENT_WINDOWS: usize = 8;

/// The largest length or offset that fits the packing below (40 bits). Far above
/// [`TRANSFER_MAX`], so the bit-packing has never been what bounds a transfer; the bound only
/// guards the packing itself, and [`TRANSFER_MAX`] is what bounds a payload.
pub const MAX_LEN: u64 = (1 << 40) - 1;

/// **The channel's arithmetic, checked by the build rather than by a test**, because a test does
/// not run in the EL0 build and a build is the only artifact that can be wrong about this.
///
/// Both halves have a failure mode worth naming. A channel that is not a whole number of pages
/// cannot be wired at all, since a mapping is per page. And a length the packing truncates would
/// arrive at the server as a **smaller successful** read rather than as an error, which is
/// DECISIONS §27's half-working write wearing a different hat.
const _: () = {
    assert!(TRANSFER_PAGES >= 1);
    assert!(TRANSFER_MAX == TRANSFER_PAGES * super::PAGE);
    assert!(TRANSFER_MAX as u64 <= MAX_LEN);
};
/// The largest handle the packing can carry (16 bits). The server's table is far smaller.
pub const MAX_HANDLE: u64 = 0xffff;

/// Pack a file request's first word: opcode (bits 63:56), handle (bits 55:40), length (bits
/// 39:0). [`OPEN`] and [`CREATE`] pass handle 0 and length = the name length;
/// [`READ`]/[`WRITE`] pass the handle and the byte count; [`CLOSE`]/[`FSTAT`]/[`TRUNCATE`] pass
/// the handle and length 0 ([`TRUNCATE`]'s new size rides in the second word).
///
/// # Examples
///
/// Control needs no shared page; the page is for the bytes.
///
/// ```
/// use filesystem_protocol::{fs, operation};
///
/// // `read(handle 3, 1024)`.
/// let w0 = fs::req(fs::READ, 3, 1024);
/// assert_eq!(operation(w0), fs::READ); // the opcode field is the same one in both protocols
/// assert_eq!(fs::req_handle(w0), 3);
/// assert_eq!(fs::req_len(w0), 1024);
///
/// // An OPEN sends handle ROOT and the *name's* length; the name itself is in the page.
/// let w0 = fs::req(fs::OPEN, fs::ROOT, b"report.txt".len() as u64);
/// assert_eq!(fs::req_handle(w0), fs::ROOT);
/// assert_eq!(fs::req_len(w0), 10);
/// ```
pub const fn req(operation: u64, handle: u64, len: u64) -> u64 {
    (operation << super::OPERATION_SHIFT) | ((handle & MAX_HANDLE) << 40) | (len & MAX_LEN)
}
/// The handle of a file request word.
pub const fn req_handle(w0: u64) -> u64 {
    (w0 >> 40) & MAX_HANDLE
}
/// The length/count of a file request word.
pub const fn req_len(w0: u64) -> usize {
    (w0 & MAX_LEN) as usize
}

/// Pack [`RENAME`]'s **destination** half into the second word: the same handle and length
/// fields [`req`] uses, minus the opcode, because a rename names two directories and one word
/// cannot carry both. Read back with [`dst_handle`] and [`dst_len`].
pub const fn rename_dst(handle: u64, len: u64) -> u64 {
    ((handle & MAX_HANDLE) << 40) | (len & MAX_LEN)
}
/// The destination directory handle of a [`RENAME`]'s second word.
pub const fn dst_handle(w1: u64) -> u64 {
    (w1 >> 40) & MAX_HANDLE
}
/// The destination name's length, in bytes, of a [`RENAME`]'s second word.
pub const fn dst_len(w1: u64) -> usize {
    (w1 & MAX_LEN) as usize
}

#[cfg(test)]
mod tests;
