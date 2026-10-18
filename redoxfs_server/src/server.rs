//! **The file service itself**: one RedoxFS image on one [`Disk`], its open-handle table, and every
//! verb the wire's operations reach.
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

use super::*;

/// What one handle names, and what may be done through it.
///
/// The rights ride on the handle rather than on the session because that is what makes descending
/// mean anything: a child directory handle is a *different* authority from its parent, held at the
/// same time by the same client, and a file opened under it inherits its direction. A session-wide
/// rights field could not express any of that.
#[derive(Clone, Copy)]
pub(crate) enum Entry {
    /// A directory. Its rights are the full ladder.
    Dir(TreePtr<Node>, Rights),
    /// A file, carrying the [`dir::READ`]/[`dir::WRITE`] bits of the directory it was opened under.
    /// The other rungs are meaningless on a file and are simply never asked about.
    File(TreePtr<Node>, Rights),
}

/// A file service over one RedoxFS image on one [`Disk`]. Generic over the disk so the host tests
/// drive a `DiskMemory`/`DiskFile` and the EL0 binary drives the block-IPC client, with identical
/// filesystem code between them.
pub struct Server<D: Disk> {
    pub(crate) fs: FileSystem<D>,
    /// The open-handle table. `handles[h]` is what a handle names, or `None` for a freed slot; the
    /// index is the handle. A capability the server issues and checks, never trusts.
    ///
    /// **Slot 0 is the bound directory** ([`filesystem_contract::fs::ROOT`]), installed by [`Server::open`]
    /// before anything is served and never closeable. It exists so the directory an endpoint
    /// designates is an ordinary handle: every name resolves under a handle, and there is no
    /// separate "the current directory" field for a verb to forget to consult.
    handles: Vec<Option<Entry>>,
    /// A monotonically increasing stand-in for wall-clock seconds, so a write advances the node's
    /// mtime deterministically. The server has no RTC; the value only needs to move forward, and the
    /// host tests assert on file *contents*, not timestamps.
    clock: u64,
    /// Listings and small files, remembered until something changes them (milestone 606). Only
    /// [`Server::look`], [`Server::change`] and [`Server::change_file`] open a transaction, and
    /// the last two are what keep this honest; `memo.rs` has the rule.
    memo: Memo,
    /// The badge that minted each handle, beside [`Server::handles`] and indexed the same way;
    /// 0 for a handle no bound badge minted (milestone 606, ruling D).
    owners: Vec<u64>,
    /// Which badges are scoped to a subtree grant (ruling D, `subtree_scope`).
    bindings: subtree_scope::Bindings<{ filesystem_contract::fs::CLIENT_WINDOWS }>,
}

impl<D: Disk> Server<D> {
    /// Open an existing RedoxFS image and bind the service to its root directory.
    ///
    /// **What recovers a crashed image, precisely** (a correction, milestone 37). This used to say
    /// that `cleanup: true` "replays the header ring to the newest consistent generation". It does
    /// not, and the distinction matters because it names the wrong mechanism. `FileSystem::open`
    /// scans all 256 header-ring slots **unconditionally**, keeps the newest one whose seahash still
    /// checks out, and ignores the rest; that scan is the crash recovery, and it happens whatever
    /// `cleanup` is. What `cleanup: true` adds is a tidy-up on top: release the nodes nothing points
    /// at any more, and commit. The server passes it because a mount should not leak, not because
    /// the recovery depends on it.
    ///
    /// The recovery is measured rather than described: `tests/crash_consistency.rs` cuts the device
    /// at every point in a workload, tears the write it was in the middle of, and mounts the result
    /// through this function. See [`crate::crash`] and DECISIONS §34.
    ///
    /// The server never opens with the creation APIs: those are std-gated and stay host-side (the
    /// server opens, it never makes).
    pub fn open(disk: D) -> Result<Self> {
        let fs = FileSystem::open(disk, None, None, true)?;
        Ok(Self {
            fs,
            // Handle 0 is the bound directory, and it carries every right: this server IS the whole
            // filesystem's authority, and narrowing happens at the endpoint above it, never here.
            // `Rights::root` is the only constructor that invents a rights set, and this is its
            // only caller.
            handles: alloc::vec![Some(Entry::Dir(TreePtr::root(), Rights::root(dir::ALL)))],
            clock: 1,
            memo: Memo::default(),
            owners: alloc::vec![0],
            bindings: subtree_scope::Bindings::new(),
        })
    }

    /// Resolve `name` under the **bound directory** and return a handle for it: the shorthand for
    /// [`Server::open_file_at`] with [`filesystem_contract::fs::ROOT`], which is what every caller that predates
    /// directory handles means.
    pub fn open_file(&mut self, name: &NameCopy) -> Result<u32> {
        self.open_file_at(filesystem_contract::fs::ROOT as u32, name)
    }

    /// Resolve `name` under the directory `handle` names and return a file handle for it.
    ///
    /// The name is a single component resolved in that directory; it is not a path, which is the
    /// whole point (no walk, no escape). `EINVAL` for anything that is not one component,
    /// [`dir::EISDIR`] if the name is a directory ([`Server::open_dir`] is the verb for that), and
    /// `ENOENT` if there is no such entry **or** if the directory carries neither [`dir::READ`] nor
    /// [`dir::WRITE`]: a holder that may not reach a name must not learn that the name is there.
    ///
    /// The file handle **inherits** the directory's read and write bits, which is what makes
    /// milestone 47's "a program handed a directory to write logs into" mean something: the
    /// direction was decided when the directory was granted, and the file cannot widen it.
    pub fn open_file_at(&mut self, handle: u32, name: &NameCopy) -> Result<u32> {
        self.open_file_path(handle, name, 0)
    }

    /// [`Server::open_file_at`], where `name` may be a **relative path** (milestone 606, calef's
    /// ruling A on #1387, 2026-09-27): `a/b/f` is resolved one component at a time, exactly as the
    /// hop-by-hop walk a client used to send, and nothing else. Each directory on the way is an
    /// [`Server::open_dir`] asking for `DESCEND | requested`: it needs `DESCEND` on its parent,
    /// narrows to that request, and refuses with `EPERM` if it would come up short. So the file
    /// handle carries exactly the bits the hop-by-hop walk would have given it. Every component
    /// passes `check_component`, so `..` and empty steps are still `EINVAL`. `requested` is
    /// `OPEN`'s second word; a single-component name ignores it, as `OPEN` always has.
    pub fn open_file_path(&mut self, handle: u32, name: &NameCopy, requested: u64) -> Result<u32> {
        let (parent, rights, name) = self.walk_prefix(handle, name, requested)?;
        if rights.denies_all(dir::READ | dir::WRITE) {
            return Err(Error::new(syscall::error::ENOENT));
        }
        let (ptr, kind) = self.lookup(parent, name)?;
        if kind == Kind::Directory {
            return Err(Error::new(EISDIR));
        }
        // A file, and never a symbolic link or a mount point: `subtree_scope` owns that rule.
        subtree_scope::landing(kind, false).map_err(refused)?;
        // Register the node as open with the engine's usage table. This is what makes `unlink` an
        // unlink: without it, removing the last name frees the node immediately and this handle
        // would then read a deallocated one. See `Server::unlink`.
        //
        // **Done here rather than by `Transaction::on_open_node`**, which is these three lines
        // inside a transaction, and opening a transaction clones the engine's whole allocator: that
        // clone was most of the server's remaining time per request once the memo answered the
        // lookup (milestone 606, `examples/walk_replay.rs`). The table is the engine's own public
        // field and nothing else about the open touches the image.
        let uses = self.fs.node_usages.entry(ptr.id()).or_insert(0);
        *uses = uses.checked_add(1).ok_or(Error::new(EINVAL))?;
        Ok(self.install(Entry::File(ptr, rights.attenuate(dir::READ | dir::WRITE))))
    }

    /// **Descend: resolve `name` under the directory `handle` names and hand back a handle to the
    /// child directory, carrying `requested` rights** (milestone 47's keystone).
    ///
    /// The child's rights are `parent & requested`, and [`Rights::attenuate`] is the only way this
    /// crate makes a rights set that is not a root's, so **a child can never carry a right its
    /// parent lacked** and no descendant of it can either. The `EPERM` below is not what enforces
    /// that: delete it and the intersection still holds. What it does is refuse to hand back
    /// *less* than was asked for without saying so, which is DECISIONS §42's rule against silent
    /// degradation.
    ///
    /// Needs [`dir::DESCEND`] on the parent, and its absence answers `ENOENT` rather than a
    /// refusal, so a holder that may not walk in cannot map what is there.
    pub fn open_dir(&mut self, handle: u32, name: &NameCopy, requested: u64) -> Result<u32> {
        // A relative path (milestone 606, ruling A): every directory before the last is a descent
        // asking for `DESCEND | requested`, the shape the hop-by-hop walk sends.
        let (parent, rights, name) = self.walk_prefix(handle, name, requested)?;
        let (ptr, rights) = self.child_dir(parent, rights, name, dir::DESCEND)?;
        let granted = rights.attenuate(requested);
        if granted != Rights::root(requested) {
            return Err(Error::new(EPERM));
        }
        Ok(self.install(Entry::Dir(ptr, granted)))
    }

    /// **Make a child directory and hand back a capability to it**: `mkdir` as descend-with-creation,
    /// which is why it lives beside [`Server::open_dir`] and shares its rights arithmetic exactly.
    ///
    /// Needs [`dir::CREATE`] *and* [`dir::DESCEND`] on the parent: making a directory you could not
    /// have walked into would be a way to mint a capability out of a right that was withheld.
    /// `EEXIST` if the name is already there, and nothing is modified, for the reason
    /// [`Server::create_file`] gives.
    pub fn make_dir(&mut self, handle: u32, name: &NameCopy, requested: u64) -> Result<u32> {
        let name = name.component()?;
        let (parent, rights) = self.dir_at(handle)?;
        if !rights.allows(dir::CREATE) {
            return Err(Error::new(EROFS));
        }
        if !rights.allows(dir::DESCEND) {
            return Err(Error::new(EPERM));
        }
        let granted = rights.attenuate(requested);
        if granted != Rights::root(requested) {
            return Err(Error::new(EPERM));
        }
        self.clock += 1;
        let now = self.clock;
        let node = self.change(|tx| {
            if tx.find_node(parent, name).is_ok() {
                return Err(Error::new(EEXIST));
            }
            tx.create_node(parent, name, Node::MODE_DIR | 0o755, now, 0)
        })?;
        Ok(self.install(Entry::Dir(node.ptr(), granted)))
    }

    /// **Enumerate the directory `handle` names**, filling `out` with [`filesystem_contract::dirent`] records
    /// and returning how many bytes were used. `cursor` is the index of the first entry to return;
    /// 0 bytes back means the cursor is past the end.
    ///
    /// Needs [`dir::ENUMERATE`], and its absence is [`dir::EPERM`] rather than an empty listing.
    /// An empty listing would be a statement about the *directory*, which would be false; the true
    /// statement is about the capability, and DECISIONS §42 says a verb that is not offered fails
    /// loudly instead of degrading into a plausible lie.
    ///
    /// Entries are sorted by name so the cursor means the same thing across calls. **The honest
    /// caveat**: a name added or removed between two calls of one walk can be seen twice or
    /// missed. That is readdir's usual caveat and it is recorded rather than fixed, because fixing
    /// it means holding a snapshot per client and this table is not per client. Since milestone
    /// 606 the listing is memoized until the next namespace change, so between changes every page
    /// comes from one listing.
    ///
    /// # BUGS
    ///
    /// - **The first listing of a directory costs a node read per entry**; [`list`] says why. It is
    ///   paid once per directory per namespace change, not once per page.
    pub fn read_dir(&mut self, handle: u32, cursor: u32, out: &mut [u8]) -> Result<usize> {
        let (ptr, rights) = self.dir_at(handle)?;
        if !rights.allows(dir::ENUMERATE) {
            return Err(Error::new(EPERM));
        }
        if let Some(listing) = self.memo.listing(ptr) {
            return Ok(encode_listing(listing, cursor, out));
        }
        let listing = self.look(|tx| list(tx, ptr))?;
        let used = encode_listing(&listing, cursor, out);
        self.memo.keep_listing(ptr, listing);
        Ok(used)
    }

    /// Read up to `buf.len()` bytes from `handle` at `offset`, returning the count (0 at EOF). Passes
    /// atime 0 so a read never advances the node's access time (never *newer* than what is stored),
    /// which keeps reads from triggering a copy-on-write of the node on every call.
    ///
    /// `EBADF` if the handle was not opened for reading, which is POSIX's own answer for reading a
    /// write-only descriptor and is a fact about the handle rather than a policy about the file.
    pub fn read(&mut self, handle: u32, offset: u64, buf: &mut [u8]) -> Result<usize> {
        let ptr = self.file_at(handle, dir::READ, EBADF)?;
        if self.memo.file(ptr).is_none() && !self.memo.is_large(ptr) {
            // First read of this file since anything changed: a small one is read whole, once.
            let body = self.look(|tx| {
                let size = tx.read_tree(ptr)?.data().size();
                if !Memo::would_keep(size) {
                    return Ok(None);
                }
                let mut body = alloc::vec![0u8; size as usize];
                let n = tx.read_node(ptr, 0, &mut body, 0, 0)?;
                body.truncate(n);
                Ok(Some(body))
            })?;
            match body {
                Some(body) => self.memo.keep_file(ptr, body),
                None => self.memo.mark_large(ptr),
            }
        }
        if let Some(body) = self.memo.file(ptr) {
            let start = (offset as usize).min(body.len());
            let n = (body.len() - start).min(buf.len());
            buf[..n].copy_from_slice(&body[start..start + n]);
            return Ok(n);
        }
        self.look(|tx| tx.read_node(ptr, offset, buf, 0, 0))
    }

    /// Write `data` to `handle` at `offset`, returning the count. Advances the internal clock so the
    /// node's mtime moves forward; the data persists regardless of the timestamp.
    ///
    /// [`dir::EROFS`] without [`dir::WRITE`], the same word and the same argument
    /// `fs_file_caretaker` uses: through this capability the file is read-only, and there is no
    /// policy that could have said yes.
    ///
    /// `EFBIG` when `offset + data.len()` does not fit in 64 bits or passes [`MAX_FILE_END`],
    /// checked before the engine sees it. RedoxFS adds the two unchecked, and the wrapped end made
    /// its inline-data path slice backwards and panic, which killed the server; an end past the
    /// tree indexed past it (both found by the `redoxfs_server_session` fuzz target).
    pub fn write(&mut self, handle: u32, offset: u64, data: &[u8]) -> Result<usize> {
        let ptr = self.file_at(handle, dir::WRITE, EROFS)?;
        match offset.checked_add(data.len() as u64) {
            Some(end) if end <= MAX_FILE_END => {}
            _ => return Err(Error::new(EFBIG)),
        }
        self.clock += 1;
        let now = self.clock;
        self.change_file(ptr, |tx| tx.write_node(ptr, offset, data, now, 0))
    }

    /// The current size, in bytes, of the file a handle names. Allowed through any file handle: a
    /// holder that may write but not read still has to be able to find the end.
    pub fn fstat(&mut self, handle: u32) -> Result<u64> {
        let ptr = self.file_at(handle, 0, EBADF)?;
        if let Some(body) = self.memo.file(ptr) {
            return Ok(body.len() as u64);
        }
        self.look(|tx| Ok(tx.read_tree(ptr)?.data().size()))
    }

    /// Create `name` under the bound directory and return a handle for it (milestone 31 phase 2).
    ///
    /// **Semantics, stated because the absence of these verbs cost a day.** If `name` does not exist
    /// it is created empty and opened. If it *does* exist this returns `EEXIST` and creates nothing:
    /// create is create, not create-or-open, so a caller that wants either has to say which it wants.
    /// The alternative (silently opening an existing file) is what makes a partly-working write look
    /// like a working one, which is the exact failure mode §27 records.
    ///
    /// Resolved under the bound directory like every other name, so this cannot create outside the
    /// granted subtree, and `check_component` rejects a path rather than walking one.
    pub fn create_file(&mut self, name: &NameCopy) -> Result<u32> {
        self.create_file_at(filesystem_contract::fs::ROOT as u32, name)
    }

    /// [`Server::create_file`], under the directory `handle` names. Needs [`dir::CREATE`] on it;
    /// without it the answer is [`dir::EROFS`], because through this capability that directory takes
    /// no new names.
    pub fn create_file_at(&mut self, handle: u32, name: &NameCopy) -> Result<u32> {
        let name = name.component()?;
        let (parent, rights) = self.dir_at(handle)?;
        if !rights.allows(dir::CREATE) {
            return Err(Error::new(EROFS));
        }
        // Existence is checked inside the same transaction as the create, so two clients racing on
        // one name cannot both be told they created it.
        self.clock += 1;
        let now = self.clock;
        let ptr = self.change(|tx| {
            if tx.find_node(parent, name).is_ok() {
                return Err(Error::new(EEXIST));
            }
            let node = tx.create_node(parent, name, Node::MODE_FILE | 0o644, now, 0)?;
            // Registered as open exactly as `open_file_at` registers, and for its reason: a file
            // created and then unlinked with its handle still open is the temp-file idiom.
            tx.on_open_node(node.ptr())?;
            Ok(node.ptr())
        })?;
        Ok(self.install(Entry::File(ptr, rights.attenuate(dir::READ | dir::WRITE))))
    }

    /// Set the size of the file a handle names to exactly `size` bytes (milestone 31 phase 2).
    ///
    /// **Both directions, deliberately.** Shrinking discards the bytes past `size`; growing extends
    /// with zeroes. That is POSIX `ftruncate` semantics and it is what makes a shorter write able to
    /// *replace* a file's contents rather than leave the old tail behind. §27 records what the absence
    /// of this cost: a 61-byte write over a 64-byte file left a 64-byte file, three investigations read
    /// the resulting assertion failure as a write failure, and the engine was never at fault.
    ///
    /// **What is guaranteed if the machine dies mid-truncate: untested (milestone 37).** RedoxFS is
    /// copy-on-write and commits through a header ring, and the server always mounts with the
    /// `cleanup: true` replay that walks back to the newest *consistent* generation, so the engine's
    /// design says a torn truncate leaves either the old size or the new one. We have not injected a
    /// torn or dropped write to check, so that is a design claim and not a measurement. Truncate and
    /// create are the first verbs that change a file's *shape* rather than its bytes, which makes them
    /// the most interesting cases for milestone 37 to attack.
    ///
    /// # BUGS
    ///
    /// **Shrinking a file costs time linear in its logical size, sparse or not**, and this server
    /// answers one request at a time, so any client holding a writable handle (a confined one
    /// included, inside its own grant) can stall every other client with two requests: grow a file
    /// to near [`MAX_FILE_END`], which is cheap because the engine writes no zero records, then
    /// shrink it, which walks every record pointer in between. The same walk runs when such a file's
    /// last name and handle go. Found by the `redoxfs_server_session` fuzz target as a ten-second
    /// timeout (2026-10-04 UTC); not fixed here, because the two fixes are a pin divergence (skip a
    /// null subtree in `truncate_node_inner`) or a semantics choice (refuse a sparse size past the
    /// image), and both are an architect's. Proposed as
    /// `design/roadmap/0772-a-client-cannot-stall-the-file-server-with-a-sparse-file.md`.
    pub fn truncate(&mut self, handle: u32, size: u64) -> Result<()> {
        // A truncate carries no bytes, so a guard that only covered `write` would leave a way to
        // destroy a file just as thoroughly. It takes the same right and answers the same word.
        let ptr = self.file_at(handle, dir::WRITE, EROFS)?;
        // A file grown past the tree reads past it later; [`MAX_FILE_END`] has the defect.
        if size > MAX_FILE_END {
            return Err(Error::new(EFBIG));
        }
        self.clock += 1;
        let now = self.clock;
        self.change_file(ptr, |tx| tx.truncate_node(ptr, size, now, 0))
    }

    /// **Move `src` under the directory `src_dir` names to `dst` under the directory `dst_dir`
    /// names** (milestone 47). The verb `mv` is, and the rung that gave [`dir::REMOVE`] something to
    /// enforce.
    ///
    /// # The two atomicities, which POSIX taught everyone to conflate (DECISIONS §42)
    ///
    /// - **Concurrency-atomic.** Nothing observes the state where the name is in both places or
    ///   neither, and the reason is structural rather than a lock: the serve loop runs one request
    ///   to completion before it receives the next, so inside this server there is no concurrent
    ///   observer at all.
    /// - **Crash-atomic.** Everything below happens inside one `fs.tx`, which reaches the platter
    ///   through one commit in RedoxFS's header ring, so a fresh mount finds the old name or the new
    ///   one. Measured rather than asserted: this is the last operation of the workload in
    ///   `tests/crash_consistency.rs`, whose sweep cuts the device at every write the workload
    ///   makes, so the cut lands inside this transaction too.
    ///
    /// # Rights
    ///
    /// [`dir::REMOVE`] on the source (its name goes away) and [`dir::CREATE`] on the destination (a
    /// name appears), each refused with [`dir::EROFS`]. Within one directory a rename needs both on
    /// that one. **Checked before anything is resolved**, so a capability that may not move a name
    /// cannot use this verb to find out whether one is there.
    ///
    /// # What it refuses, and why each refusal is the contract's rather than the backend's
    ///
    /// - A **directory** may be renamed in place but not moved between directories: `EINVAL`. See
    ///   [`filesystem_contract::fs::RENAME`] for the cycle argument. Keeping the parent unchanged is what makes
    ///   the in-place case provably safe without an ancestry walk.
    /// - Replacing a destination of a **different kind** is `EISDIR` (file over directory) or
    ///   `ENOTDIR` (directory over file). RedoxFS's own `rename_node` allows both, because it passes
    ///   the *destination's* type to `remove_node` and so never compares the two; these two lines
    ///   are ours, at our boundary, for §42's reason that an offered verb must mean one thing on
    ///   every backend.
    /// - A non-empty destination directory is `ENOTEMPTY`, from the engine.
    /// - Either name that is not a single component is `EINVAL`, so `..` means nothing here either.
    pub fn rename(
        &mut self,
        src_dir: u32,
        src: &NameCopy,
        dst_dir: u32,
        dst: &NameCopy,
    ) -> Result<()> {
        let src = src.component()?;
        let dst = dst.component()?;
        let (src_parent, src_rights) = self.dir_at(src_dir)?;
        let (dst_parent, dst_rights) = self.dir_at(dst_dir)?;
        if !src_rights.allows(dir::REMOVE) || !dst_rights.allows(dir::CREATE) {
            return Err(Error::new(EROFS));
        }
        let same_dir = src_parent.id() == dst_parent.id();
        self.change(|tx| {
            let node = tx.find_node(src_parent, src)?;
            if node.data().is_dir() && !same_dir {
                return Err(Error::new(EINVAL));
            }
            // The node a replaced destination names, when this rename is its last link. Its
            // extended attributes have to go with it, in **this** transaction: `rename_node` calls
            // `remove_node` on the destination and throws the freed id away, so nothing else here
            // ever learns that a node died, and a leaked blob would be inherited by whatever node
            // the engine hands that id to next. Attributes keyed on a node are free across a rename
            // and dangerous across a reuse, which are two halves of the same property.
            let mut replaced = None;
            if let Ok(existing) = tx.find_node(dst_parent, dst) {
                // A no-op rename is a success, not a self-replacement: `remove_node` would take the
                // only link and `link_node` would then have nothing to point at.
                if existing.id() == node.id() {
                    return Ok(());
                }
                if existing.data().is_dir() != node.data().is_dir() {
                    return Err(Error::new(if existing.data().is_dir() {
                        EISDIR
                    } else {
                        ENOTDIR
                    }));
                }
                if existing.data().links() == 1 {
                    replaced = Some(existing.id());
                }
            }
            tx.rename_node(src_parent, src, dst_parent, dst)?;
            // The source keeps its node and therefore keeps its attributes, with no code here
            // knowing a rename happened. That is the whole reason the store keys on a node rather
            // than on a path, and it is what an AppleDouble sidecar gets wrong.
            if let Some(id) = replaced {
                purge_attrs(tx, id)?;
            }
            Ok(())
        })
    }

    /// **Remove `name` from the directory `handle` names: `rm`'s verb, and it is `unlink` rather
    /// than revocation** (milestone 47's commands).
    ///
    /// Needs [`dir::REMOVE`], refused with [`dir::EROFS`], and the right is checked **before the
    /// name is resolved**, so a capability that may not remove cannot use this verb to discover
    /// whether a name is there. `ENOENT` if there is no such entry, `EINVAL` if it is not a single
    /// component, and [`dir::EISDIR`] for a directory: `rm` takes a name of a file away, and this
    /// contract has no `RMDIR`, which is stated where a caller meets it
    /// ([`filesystem_contract::fs::UNLINK`]) rather than approximated by removing whatever is found.
    ///
    /// # What "unlink, not revoke" costs, and where it is paid
    ///
    /// The name goes and the object stays while anyone still holds a handle on it, which is POSIX's
    /// promise and the thing atomic-replace depends on. It is not free and it is not automatic:
    /// RedoxFS frees a node the moment its last link goes **unless the node is registered as open**
    /// in its usage table, in which case `remove_node` queues it on the release list and
    /// `on_close_node` collects it later. So [`Server::open_file_at`] and [`Server::create_file_at`]
    /// register, [`Server::close`] deregisters, and *that* pairing is what makes this verb an
    /// unlink. Without it the read that follows an unlink reads a deallocated node, and `rm` would
    /// be a revoke wearing an unlink's name, which is exactly the conflation §42 forbids.
    ///
    /// **BUGS.** A handle that is never closed pins the node for the life of the server, exactly as
    /// a leaked fd does on Unix, and this handle table is per *server* and outlives its clients: a
    /// confined program that dies mid-session leaves its files on the release list forever. The fix
    /// is per-client handle ownership, which the note records as the same missing thing that makes
    /// revocation unavailable.
    pub fn unlink(&mut self, handle: u32, name: &NameCopy) -> Result<()> {
        let name = name.component()?;
        let (parent, rights) = self.dir_at(handle)?;
        if !rights.allows(dir::REMOVE) {
            return Err(Error::new(EROFS));
        }
        // `remove_node` compares the node's kind against the mode it is given, so `EISDIR` for a
        // directory is the engine's own answer here and matches POSIX. Contrast `rename`, where the
        // kind check had to be ours because the engine never compares the two.
        //
        // It also answers `Some(id)` **exactly when the node's last link went**, which is precisely
        // when its extended attributes must go too, and `None` when a link remains. So the purge
        // asks the engine rather than guessing, and it happens inside the same transaction as the
        // removal: a crash cannot leave a freed node's attributes behind for the next node to
        // inherit that id (milestone 57, DECISIONS §34).
        self.change(|tx| {
            if let Some(id) = tx.remove_node(parent, name, Node::MODE_FILE)? {
                purge_attrs(tx, id)?;
            }
            Ok(())
        })
    }

    /// **Remove an empty directory `name` from the directory `handle` names: `rmdir(2)`, and
    /// empty-only is the entire safety property** (milestone 47's `rm -r`).
    ///
    /// Needs [`dir::REMOVE`] on the parent, refused with [`dir::EROFS`], checked **before the name
    /// is resolved** for [`Server::unlink`]'s reason. `ENOTEMPTY` if anything is still in it,
    /// `ENOTDIR` for a file (POSIX's own answer, and the mirror of `unlink`'s `EISDIR`), `EINVAL`
    /// for anything that is not a single component, `ENOENT` if there is no such name.
    ///
    /// # Why this verb is safe to offer and a recursive one is not
    ///
    /// `mkdir` shipped in the commands lane with nothing that removed what it made, and the
    /// objection recorded there was that "a verb that removes whatever it finds is how one word
    /// takes a subtree away". That is right about a *recursive* verb and is not right about Unix's,
    /// which is the point: **no single call on this contract can take a subtree away.** The
    /// recursion lives in userspace (`components/src/rm.rs`) as a loop of individually safe steps, each of
    /// which needs the rights for it at its own level, so a walk stops exactly where the
    /// capabilities stop rather than where a check in here remembered to look.
    ///
    /// # It is not revocation, and here is what that costs
    ///
    /// [`Server::unlink`] pairs with the engine's usage table so an unlinked file survives for
    /// whoever holds a handle. **Directory handles are not registered that way**, so a client
    /// holding a handle to the directory this call removes is left naming a node the engine has
    /// freed. ~~Its next request through that handle fails (`ENOENT`) rather than reaching
    /// something else.~~ **That was wrong, and milestone 633 (an outside agent attacks the
    /// confinement claim)'s second pass corrected it (2026-10-05 UTC, read from
    /// `vendor/redoxfs/src/transaction.rs`, not run):** `release_node` frees the tree slot at once
    /// when its usage count is zero, and `insert_tree` hands the first free slot to the next node
    /// created anywhere on the image. So the stale handle names whichever directory next takes that
    /// id, with the stale handle's rights, which is a reach outside the holder's grant rather than
    /// an `ENOENT`. Removing a name is still not revoking a capability: the handle table is per
    /// *server* and this server cannot enumerate the clients holding handles, so it could not
    /// invalidate them if it wanted to. Recorded rather than fixed, because registering directory
    /// handles is the same change as per-client handle ownership, which is what revocation needs.
    pub fn rmdir(&mut self, handle: u32, name: &NameCopy) -> Result<()> {
        let name = name.component()?;
        let (parent, rights) = self.dir_at(handle)?;
        if !rights.allows(dir::REMOVE) {
            return Err(Error::new(EROFS));
        }
        // `remove_node` compares the node's kind against the mode it is given, so `ENOTDIR` for a
        // file is the engine's own answer, and it refuses a non-empty directory with `ENOTEMPTY`
        // from inside the same transaction that would have removed it. Both are POSIX's words for
        // `rmdir`, which is why neither is checked again out here: a second opinion that could
        // disagree with the engine is worse than one that cannot.
        //
        // A directory carries extended attributes exactly as a file does, so it is purged exactly as
        // a file is. See [`Server::unlink`] for why the engine's `Some(id)` is what decides it.
        self.change(|tx| {
            if let Some(id) = tx.remove_node(parent, name, Node::MODE_DIR)? {
                purge_attrs(tx, id)?;
            }
            Ok(())
        })
    }

    // --- mtime (milestone 47's `touch`, DECISIONS §112) ---
    //
    // Three methods, one per `filesystem_contract::fs` verb, and all three resolve `name` directly
    // under the directory `handle` names rather than taking an already-open file handle: `touch`
    // never opens what it acts on ([`Server::unlink`]'s shape, not [`Server::write`]'s). That also
    // sidesteps a real constraint rather than a stylistic one: a **file** handle's `Rights` are
    // hard-attenuated to `dir::READ | dir::WRITE` the moment [`Server::open_file_at`] or
    // [`Server::create_file_at`] mints it, so `dir::SETTIME` could never be checked against one.
    // Only a directory handle's rights carry the full seven-rung ladder, so these three check the
    // *parent's* rights, exactly as [`Server::unlink`] and [`Server::rmdir`] do.

    /// **The modification time of `name`, in Unix seconds** (`filesystem_contract::fs::GETMTIME`).
    ///
    /// Needs [`dir::READ`], and its absence is `ENOENT` rather than a mutating-right's `EROFS`:
    /// this is a naming right, [`Server::open_file_at`]'s own rule, checked before the name is
    /// resolved so a holder that may not read a time cannot use this to learn whether a name is
    /// there. Works on a directory exactly as on a file, [`Server::get_xattr`]'s reason.
    pub fn mtime(&mut self, handle: u32, name: &NameCopy) -> Result<u64> {
        let name = name.component()?;
        let (parent, rights) = self.dir_at(handle)?;
        if !rights.allows(dir::READ) {
            return Err(Error::new(ENOENT));
        }
        let node = self.look(|tx| tx.find_node(parent, name))?;
        Ok(node.data().mtime().0)
    }

    /// **Set `name`'s modification time to now** (`filesystem_contract::fs::SETMTIME`, bare `touch`).
    ///
    /// "Now" is this server's own advancing logical clock, the exact value every mutating verb
    /// already stamps a node with ([`Server::write`], [`Server::create_file_at`],
    /// [`Server::truncate`]), **not** a reading of a real wall clock: this server has no RTC wired
    /// to it (the same gap the `clock` field on this struct records). A `touch` on two names in
    /// sequence is guaranteed to
    /// observe the second mtime greater than the first, which is what a staleness check actually
    /// depends on; it is not guaranteed to agree with a wall-clock reading taken at the same
    /// instant. See `filesystem_contract::fs::SETMTIME`'s doc and notes/touch.md's `BUGS`.
    ///
    /// Needs [`dir::WRITE`], refused with [`dir::EROFS`], checked before the name is resolved for
    /// [`Server::unlink`]'s reason. No more than [`dir::WRITE`], because the value recorded is one
    /// the server observed rather than one the caller asserted (DECISIONS §112).
    pub fn set_mtime_now(&mut self, handle: u32, name: &NameCopy) -> Result<()> {
        let name = name.component()?;
        let (parent, rights) = self.dir_at(handle)?;
        if !rights.allows(dir::WRITE) {
            return Err(Error::new(EROFS));
        }
        self.clock += 1;
        let now = self.clock;
        self.change(|tx| {
            let mut node = tx.find_node(parent, name)?;
            node.data_mut().set_mtime(now, 0);
            tx.sync_tree(node)
        })
    }

    /// **Set `name`'s modification time to a caller-supplied Unix-seconds value**
    /// (`filesystem_contract::fs::SETMTIME_AT`, `touch -t`; DECISIONS §112: the ability to *lie about
    /// history*).
    ///
    /// Needs [`dir::WRITE`] **and** [`dir::SETTIME`], refused with [`dir::EROFS`] for either
    /// missing, checked before the name is resolved for [`Server::unlink`]'s reason. Unlike
    /// [`Server::set_mtime_now`], the value here is the caller's assertion rather than anything
    /// this server observed, which is exactly the authority DECISIONS §112 declines to fold into
    /// plain [`dir::WRITE`]. No range check on `seconds`: any value is accepted, because asserting
    /// an implausible history is the same authority as asserting a merely surprising one.
    pub fn set_mtime_at(&mut self, handle: u32, name: &NameCopy, seconds: u64) -> Result<()> {
        let name = name.component()?;
        let (parent, rights) = self.dir_at(handle)?;
        if !rights.allows(dir::WRITE | dir::SETTIME) {
            return Err(Error::new(EROFS));
        }
        self.change(|tx| {
            let mut node = tx.find_node(parent, name)?;
            node.data_mut().set_mtime(seconds, 0);
            tx.sync_tree(node)
        })
    }

    // --- Extended attributes (milestone 57, DECISIONS §34's 2026-07-31 amendment) ---
    //
    // The engine has none of this. The four methods below are a **layer**: a store the server keeps
    // in the image, keyed on the node's `TreePtr` id, published as `filesystem_contract::xattr::store`. What
    // makes the layer legitimate here and worthless on Linux is that nothing can bypass it, because
    // every path to these bytes goes through `filesystem_contract`. See notes/xattr.md.

    /// **Read one extended attribute of the node `handle` names**, returning its type code and the
    /// number of bytes written into `out`.
    ///
    /// Needs [`dir::READ`], refused with `EBADF`, which is what [`Server::read`] needs and answers.
    /// `ENODATA` if the attribute is not set: the only error that means absence, which is the
    /// discipline milestone 37's harness learned the hard way (notes/fs-server.md).
    ///
    /// **`ERANGE` if `out` is shorter than the value**, never a short read. POSIX's own answer, and
    /// DECISIONS §42's rule: a value clipped to fit would hand back a file that looked intact and
    /// was not. In the running system `out` is the shared page, which is larger than
    /// [`xattr::MAX_VALUE`] by construction, so this cannot fire on the wire.
    pub fn get_xattr(&mut self, handle: u32, name: &[u8], out: &mut [u8]) -> Result<(u32, usize)> {
        if !xattr::is_valid_name(name) {
            return Err(Error::new(xattr::ERANGE));
        }
        let ptr = self.node_at(handle, dir::READ, EBADF)?;
        self.look(|tx| {
            let blob = read_attrs(tx, ptr.id())?;
            let (kind, value) = xattr::store::get(&blob, name).ok_or(Error::new(xattr::ENODATA))?;
            if out.len() < value.len() {
                return Err(Error::new(xattr::ERANGE));
            }
            out[..value.len()].copy_from_slice(value);
            Ok((kind, value.len()))
        })
    }

    /// **Set one extended attribute on the node `handle` names.** Creating and replacing are one
    /// operation ([`filesystem_contract::fs::SETXATTR`] says why).
    ///
    /// Needs [`dir::WRITE`], refused with [`dir::EROFS`], which is what [`Server::write`] needs and
    /// answers. The three ceilings answer their own words ([`xattr::ERANGE`], [`xattr::E2BIG`],
    /// [`xattr::ENOSPC`]) so a caller can tell which one it hit.
    ///
    /// **Crash-atomic with the file's own bytes**, because the read, the rewrite and the store
    /// write all happen inside one `fs.tx`, which reaches the platter as one commit in RedoxFS's
    /// header ring. That was the check that had to pass before this layer was viable at all
    /// (DECISIONS §34), and it is why an attribute cannot survive a crash that lost the file or
    /// vice versa.
    pub fn set_xattr(&mut self, handle: u32, name: &[u8], kind: u32, value: &[u8]) -> Result<()> {
        // Checked before anything is allocated or opened, so a client cannot make the server size a
        // buffer from a length it invented. `store::set` checks again, and that is the copy the host
        // tests exercise; this one is the boundary.
        if !xattr::is_valid_name(name) {
            return Err(Error::new(xattr::ERANGE));
        }
        if value.len() > xattr::MAX_VALUE {
            return Err(Error::new(xattr::E2BIG));
        }
        let ptr = self.node_at(handle, dir::WRITE, EROFS)?;
        self.clock += 1;
        let now = self.clock;
        self.change(|tx| {
            let blob = read_attrs(tx, ptr.id())?;
            // Replacing can only shrink, so the old blob plus one whole new record is always enough
            // and `store::set` never has to refuse for want of room.
            let mut out =
                alloc::vec![0u8; blob.len() + xattr::store::record_len(name.len(), value.len())];
            let n = xattr::store::set(&blob, name, kind, value, &mut out).map_err(Error::new)?;
            write_attrs(tx, ptr.id(), &out[..n], now)
        })
    }

    /// **List the attribute names on the node `handle` names**, filling `out` with
    /// [`xattr::list`] records and returning the bytes used. Needs [`dir::READ`], refused with
    /// `EBADF`.
    ///
    /// No cursor, unlike [`Server::read_dir`]: a complete listing fits one page by construction
    /// ([`xattr::MAX_COUNT`]), so it cannot be observed half-changed the way a cursored directory
    /// walk can.
    pub fn list_xattr(&mut self, handle: u32, out: &mut [u8]) -> Result<usize> {
        let ptr = self.node_at(handle, dir::READ, EBADF)?;
        self.fs
            .tx(|tx| Ok(xattr::store::list(&read_attrs(tx, ptr.id())?, out)))
    }

    /// **Remove one extended attribute from the node `handle` names.** Needs [`dir::WRITE`],
    /// refused with [`dir::EROFS`]. `ENODATA` if it was not set, rather than a quiet success: the
    /// caller asked about one specific thing.
    pub fn remove_xattr(&mut self, handle: u32, name: &[u8]) -> Result<()> {
        if !xattr::is_valid_name(name) {
            return Err(Error::new(xattr::ERANGE));
        }
        let ptr = self.node_at(handle, dir::WRITE, EROFS)?;
        self.clock += 1;
        let now = self.clock;
        self.change(|tx| {
            let blob = read_attrs(tx, ptr.id())?;
            let mut out = alloc::vec![0u8; blob.len()];
            let n = xattr::store::remove(&blob, name, &mut out).map_err(Error::new)?;
            write_attrs(tx, ptr.id(), &out[..n], now)
        })
    }

    /// The disk under the filesystem, for a host test that counts what reached it
    /// (`walk_model::Counting`). The serve loop never needs it.
    #[cfg(feature = "hosttest")]
    pub fn disk_mut(&mut self) -> &mut D {
        &mut self.fs.disk
    }

    /// Take the disk back out of the server, **dropping the mount without unmounting**, which is
    /// exactly how a dying process leaves it: no flush, no clean shutdown, whatever is on the platter
    /// is the whole story. Milestone 37's harness uses it to get the recorded write log back after a
    /// workload; the crash tests then reconstruct the platter from that log alone.
    pub fn into_disk(self) -> D {
        self.fs.disk
    }

    /// Release a handle. `EBADF` if it was not open.
    ///
    /// **The bound directory cannot be closed**, and the refusal is `EINVAL` rather than `EBADF`:
    /// handle 0 is not something the client opened, it is the root of its namespace, and a client
    /// that could close it could make every later request in the session fail.
    ///
    /// **Nor can a bound badge's grant root, while it is bound**, for the same reason one level
    /// down, and with `EINVAL` for the same reason. The handle table is per server, so an open
    /// client could otherwise close the handle a bound badge's `ROOT` resolves to, and the next
    /// open anywhere would reuse the slot: the bound badge's `ROOT` would then name that object,
    /// outside its grant. [`Server::unbind`] revokes the badge first and then closes the root, so
    /// taking the grant back is how the handle is released. Found by the `redoxfs_server_session`
    /// fuzz target (`lane/fuzz-service-handlers`, 2026-10-04 UTC).
    ///
    /// Closing a **file** handle is also what finally frees a node whose last name was unlinked
    /// while this handle was open (see [`Server::unlink`]). Nothing else collects it, so a close is
    /// not merely bookkeeping in a table this server owns.
    pub fn close(&mut self, handle: u32) -> Result<()> {
        if handle as u64 == filesystem_contract::fs::ROOT || self.is_grant_root(handle) {
            return Err(Error::new(EINVAL));
        }
        let entry = self.entry(handle)?;
        self.handles[handle as usize] = None;
        if let Entry::File(ptr, _) = entry {
            // **The one transaction outside `look`/`change`, and why the memo survives it.**
            // Closing the last handle to an unlinked file frees its node, which is a change, but
            // not one the memo can observe: the unlink already forgot every listing, no handle
            // names the node once this returns, and its id can only be reused by a verb that
            // creates a node, which goes through `change` and forgets everything. Forgetting the
            // file's bytes here instead would make every walk re-read every file it just read.
            //
            // `Transaction::on_close_node` without the transaction when it has nothing to do, for
            // the reason `open_file_at` gives: drop the use; a node still in use, or one never
            // registered, is done. Otherwise the engine's own call runs, and it has something to
            // do only if the release list (nodes unlinked while open) is non-empty.
            match self.fs.node_usages.get_mut(&ptr.id()) {
                Some(uses) if *uses > 1 => *uses -= 1,
                Some(_) if self.fs.header.release.is_null() => {
                    self.fs.node_usages.remove(&ptr.id());
                }
                Some(_) => self.fs.tx(|tx| tx.on_close_node(ptr))?,
                None => {}
            }
        }
        Ok(())
    }

    /// **How much room the image has** ([`filesystem_contract::fs::STATFS`], milestone 54), as
    /// `(block_size, total_blocks, free_blocks)`.
    ///
    /// The handle is validated and then not used, which looks odd and is the point: it is the
    /// qualification rather than the subject. Holding any handle this server minted is what
    /// entitles a caller to ask, and the answer is about the one image behind all of them, so
    /// `EBADF` for a handle that was never minted is the whole of the check. No right is demanded;
    /// [`filesystem_contract::fs::STATFS`] argues that at length.
    ///
    /// **Where the two numbers come from, and why one is not read inside a transaction.**
    /// `header.size()` is the filesystem's size in bytes as the newest committed header records it,
    /// and `allocator.free()` counts free level-0 blocks in the *committed* allocator. Both are
    /// read outside `fs.tx`, deliberately: opening a transaction to answer a question changes
    /// nothing and would put a commit in the path of a read-only verb. RedoxFS's own `clone`
    /// computes free space from exactly this pair.
    ///
    /// A transaction in flight elsewhere would move `free`, and there is none: this server runs one
    /// request to completion before it receives the next, which is the same property
    /// [`filesystem_contract::fs::RENAME`]'s concurrency atomicity rests on.
    pub fn statfs(&mut self, handle: u32) -> Result<(u64, u64, u64)> {
        self.entry(handle)?;
        let block = redoxfs::BLOCK_SIZE;
        // Integer division, so a size that is not a whole number of blocks reports the blocks that
        // exist rather than a fraction the caller would round the wrong way.
        Ok((
            block,
            self.fs.header.size() / block,
            self.fs.allocator().free(),
        ))
    }

    /// **May this handle ask for the storage to be made durable?** (milestone 55). `Ok(())` when it
    /// may; the caller then does the IO.
    ///
    /// This is the permission half of `filesystem_contract::fs::SYNC` and **it performs no flush**, which the
    /// name is chosen to say. The flush is a `filesystem_contract::blk::FLUSH` to the block server, so it lives
    /// in the binary with the rest of the IO; RedoxFS's `Disk` trait has no sync method and this
    /// crate does not modify vendored code to add one. A method here called `sync` would be the
    /// exact kind of claim-beyond-the-mechanism milestone 55 exists to remove.
    ///
    /// `node_at` with [`dir::WRITE`], so it takes a handle of **either kind** the
    /// way `STATFS` does (the answer is about the storage, not about the node), and refuses with
    /// [`dir::EROFS`] the way every mutating verb does. The rights argument is in
    /// `filesystem_contract::fs::SYNC`'s own documentation: a sync is an action on the device rather than a
    /// fact about it, and a read-only holder has nothing outstanding to make durable.
    ///
    /// Name: provisional, minted by milestone 55's durability lane on 2026-08-18.
    pub fn sync_permitted(&self, handle: u32) -> Result<()> {
        self.node_at(handle, dir::WRITE, EROFS).map(|_| ())
    }

    /// Install an entry in the handle table, reusing a freed slot before growing. Returns the
    /// handle. Slot 0 is never free, so a minted handle is never 0 and never shadows the root.
    fn install(&mut self, entry: Entry) -> u32 {
        if let Some(i) = self.handles.iter().position(|s| s.is_none()) {
            self.handles[i] = Some(entry);
            self.owners[i] = 0;
            i as u32
        } else {
            self.handles.push(Some(entry));
            self.owners.push(0);
            (self.handles.len() - 1) as u32
        }
    }

    // --- Ruling D: subtree grants this server enforces itself (milestone 606) ---------------------

    /// **The handle a request under `badge` may use.** `subtree_scope::admit` decides: an unbound
    /// badge's handle passes through, a bound badge's `ROOT` is its grant's root and any other
    /// handle must be one it minted, and a revoked badge reaches nothing (`EBADF`).
    pub fn admit(&self, badge: u64, requested: u64) -> Result<u32> {
        let owner = match self.handles.get(requested as usize) {
            Some(Some(_)) => Some(self.owners[requested as usize]),
            _ => None,
        };
        subtree_scope::admit(self.bindings.of(badge), badge, requested, owner)
            .map(|h| h as u32)
            .map_err(refused)
    }

    /// Whether some badge's grant is rooted at `handle` (see [`Server::close`]).
    fn is_grant_root(&self, handle: u32) -> bool {
        (1..filesystem_contract::fs::CLIENT_WINDOWS as u64).any(|b| {
            self.bindings.of(b)
                == subtree_scope::Binding::Bound {
                    root: handle as u64,
                }
        })
    }

    /// Whether `badge` is bound or revoked, rather than carrying the endpoint's whole authority.
    pub fn scoped(&self, badge: u64) -> bool {
        self.bindings.of(badge) != subtree_scope::Binding::Open
    }

    /// Record that `badge` minted `handle`, if `badge` is bound. Called for every handle a verb
    /// hands back, so a bound badge can name what it opened and nothing any other client opened.
    pub fn claim(&mut self, handle: u32, badge: u64) {
        if matches!(
            self.bindings.of(badge),
            subtree_scope::Binding::Bound { .. }
        ) && let Some(owner) = self.owners.get_mut(handle as usize)
        {
            *owner = badge;
        }
    }

    /// **`BIND`: scope `badge` to the directory `handle` names**, asked for by a caller whose
    /// badge is `caller`. The rules are `subtree_scope::Bindings::bind`'s; the directory's rights
    /// are what the grant carries, fixed when the binder opened it.
    pub fn bind(&mut self, caller: u64, handle: u32, badge: u64) -> Result<()> {
        self.dir_at(handle)?;
        self.bindings
            .bind(caller, badge, handle as u64)
            .map_err(refused)
    }

    /// **`BIND_SYNC`: make `badge` sync-only** (milestone 805 (`reboot` at the prompt)), asked
    /// for by a caller whose badge is `caller`. `subtree_scope::Bindings::bind_sync_only` has the
    /// rules; [`Server::handle`] enforces what it means.
    ///
    /// Name: ratified 2026-10-06 (calef, #1783: "Approve sync for the file-server
    /// request and its capability, and flush for the device cache only.").
    pub fn bind_sync_only(&mut self, caller: u64, badge: u64) -> Result<()> {
        self.bindings.bind_sync_only(caller, badge).map_err(refused)
    }

    /// Whether `badge` may `SYNC` and nothing else.
    ///
    /// Name: ratified 2026-10-06 (calef, #1783: "Approve sync for the file-server
    /// request and its capability, and flush for the device cache only.").
    pub fn is_sync_only(&self, badge: u64) -> bool {
        self.bindings.of(badge) == subtree_scope::Binding::SyncOnly
    }

    /// **`UNBIND`: take `badge`'s grant back.** Closes every handle the badge minted and the grant's
    /// root, and leaves the badge revoked, so it reaches nothing from then on.
    pub fn unbind(&mut self, caller: u64, badge: u64) -> Result<()> {
        let root = self.bindings.unbind(caller, badge).map_err(refused)?;
        let minted: Vec<u32> = (0..self.owners.len())
            .filter(|&h| self.owners[h] == badge && self.handles[h].is_some())
            .map(|h| h as u32)
            .collect();
        for h in minted {
            self.owners[h as usize] = 0;
            let _ = self.close(h);
        }
        if root != filesystem_contract::fs::ROOT {
            let _ = self.close(root as u32);
        }
        Ok(())
    }

    /// What a handle names, or `EBADF`. Every operation reaches the table through here or through
    /// one of the two typed wrappers below, so a forged or stale handle is refused in one place.
    fn entry(&self, handle: u32) -> Result<Entry> {
        self.handles
            .get(handle as usize)
            .copied()
            .flatten()
            .ok_or(Error::new(EBADF))
    }

    /// The directory a handle names and the rights it carries. [`dir::ENOTDIR`] for a file handle,
    /// which is a fact about the handle rather than a refusal of the request.
    fn dir_at(&self, handle: u32) -> Result<(TreePtr<Node>, Rights)> {
        match self.entry(handle)? {
            Entry::Dir(ptr, rights) => Ok((ptr, rights)),
            Entry::File(..) => Err(Error::new(ENOTDIR)),
        }
    }

    /// The file a handle names, once its rights carry `needed`. `refusal` is the errno to answer
    /// when they do not, which differs by direction on purpose: a read through a write-only handle
    /// is `EBADF` (POSIX's answer, a fact about the handle) and a write through a read-only one is
    /// [`dir::EROFS`] (`fs_file_caretaker`'s answer, a fact about the capability).
    fn file_at(&self, handle: u32, needed: u64, refusal: i32) -> Result<TreePtr<Node>> {
        match self.entry(handle)? {
            Entry::File(ptr, rights) if rights.allows(needed) => Ok(ptr),
            Entry::File(..) => Err(Error::new(refusal)),
            Entry::Dir(..) => Err(Error::new(EISDIR)),
        }
    }

    /// The node a handle names, **file or directory alike**, once its rights carry `needed`.
    ///
    /// The only place in this file that does not care which kind a handle is, and extended
    /// attributes are the only thing that does not care: a directory carries them exactly as a file
    /// does, so a verb that refused one kind would be inventing a distinction the feature has not
    /// got. `refusal` differs by direction for [`Server::file_at`]'s reason.
    fn node_at(&self, handle: u32, needed: u64, refusal: i32) -> Result<TreePtr<Node>> {
        let (ptr, rights) = match self.entry(handle)? {
            Entry::Dir(ptr, rights) | Entry::File(ptr, rights) => (ptr, rights),
        };
        if rights.allows(needed) {
            Ok(ptr)
        } else {
            Err(Error::new(refusal))
        }
    }
    /// Every component of `path` but the last, descended from `handle`, and the last component.
    /// The descent is `subtree_scope::walk`, which ruling D (milestone 606, 2026-09-27) makes the one
    /// place any path is resolved: each step is exactly the `OPENDIR` a client could have sent,
    /// asking for `DESCEND | hop`, and a step through a symbolic link or a mount point is refused.
    /// This server's own naming rules (`check_component`) run on every component first, so a
    /// malformed path is `EINVAL` whatever the tree holds.
    fn walk_prefix<'n>(
        &mut self,
        handle: u32,
        path: &'n NameCopy,
        hop: u64,
    ) -> Result<(TreePtr<Node>, Rights, &'n str)> {
        let (ptr, rights) = self.dir_at(handle)?;
        // Checked here, after `dir_at`, so a bad handle answers before a bad name, as it always has.
        let path = path.path()?;
        // `subtree_scope::walk` takes bytes rather than a `NameCopy`: it is a shared crate and cannot
        // name this server's type. A deliberate exception to the typed boundary, and safe only because
        // this is its one caller here and the bytes are the checked copy's, never the window's.
        let (ptr, rights, last) =
            subtree_scope::walk(ptr, rights, path.as_bytes(), hop, |dir, name| {
                let name = core::str::from_utf8(name).map_err(|_| Fault(Error::new(EINVAL)))?;
                self.lookup(dir, name).map_err(Fault)
            })
            .map_err(|Fault(e)| e)?;
        // `last` is a suffix of `path`, which is UTF-8, cut at a `/`.
        Ok((ptr, rights, &path[path.len() - last.len()..]))
    }

    /// `name` in the directory `parent`, and what it is: from the memo when it can answer, from
    /// RedoxFS otherwise, and remembered.
    fn lookup(&mut self, parent: TreePtr<Node>, name: &str) -> Result<(TreePtr<Node>, Kind)> {
        match self.memo.lookup(parent, name) {
            Some(Some(found)) => Ok(found),
            Some(None) => Err(Error::new(ENOENT)),
            None => {
                let node = self.look(|tx| tx.find_node(parent, name))?;
                let kind = kind_of(node.data());
                self.memo.keep_found(parent, name, node.ptr(), kind);
                Ok((node.ptr(), kind))
            }
        }
    }

    /// The child directory `name` of `parent`, once `rights` allows `needed` (`ENOENT` otherwise).
    fn child_dir(
        &mut self,
        parent: TreePtr<Node>,
        rights: Rights,
        name: &str,
        needed: u64,
    ) -> Result<(TreePtr<Node>, Rights)> {
        if !rights.allows(needed) {
            return Err(Error::new(syscall::error::ENOENT));
        }
        let (ptr, kind) = self.lookup(parent, name)?;
        subtree_scope::landing(kind, true).map_err(refused)?;
        Ok((ptr, rights))
    }

    /// **A transaction that changes nothing**: every read-only verb's way in. RedoxFS commits a
    /// transaction that wrote nothing as a no-op, so this costs no device write.
    fn look<T>(&mut self, f: impl FnOnce(&mut Transaction<D>) -> Result<T>) -> Result<T> {
        self.fs.tx(f)
    }

    /// **A transaction that changes the namespace or a node**: every mutating verb's way in, and
    /// the reason [`Memo`] is never stale. It forgets everything first; forgetting is always safe,
    /// so it does not matter whether the change then succeeds.
    fn change<T>(&mut self, f: impl FnOnce(&mut Transaction<D>) -> Result<T>) -> Result<T> {
        self.memo.forget();
        self.fs.tx(f)
    }

    /// [`Server::change`] for a change to one file's bytes, which no listing depends on.
    fn change_file<T>(
        &mut self,
        ptr: TreePtr<Node>,
        f: impl FnOnce(&mut Transaction<D>) -> Result<T>,
    ) -> Result<T> {
        self.memo.forget_file(ptr);
        self.fs.tx(f)
    }
}
