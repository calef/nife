//! **`Dir`**: std's `openat`-shaped directory handle (milestone 122), unstable as
//! `#![feature(dirfd)]`.
//!
//! Cut out of `sys/fs/nife.rs` unchanged on 2026-10-12 (UTC) by lane
//! maintainer/near-ceiling-splits, for §266 (a Rust source file stays under 2,000 lines) and §282
//! (when a file is split, the parent becomes an index). What was private to `nife` is `pub(super)`
//! here, which reaches the same modules: `nife` and its children.
//!
//! Name: provisional (lane maintainer/near-ceiling-splits, 2026-10-12 UTC), for the module and the
//! file.

use super::*;

/// **A directory a program holds** (milestone 122): std's own `openat`-shaped API, `Dir::open` and
/// `Dir::open_file`, bound to the capability it already is.
///
/// This is the half of milestone 122 that matters more than nested paths, and the reason is the
/// mistake it avoids rather than the software it runs. If path composition were the only interface,
/// no program would ever be written to hold a directory and the system's model would be invisible
/// to everything running on it, which is a capability system quietly becoming an ambient one with
/// extra steps.
///
/// **It is not an interface this project invented**, which was the surprise: `std::fs::Dir` exists
/// upstream behind `#![feature(dirfd)]` (rust-lang/rust#120426) and is exactly `openat`. Until this
/// milestone nife got std's generic fallback, which stores a `canonicalize`d path, so `Dir::open`
/// on this system failed at the first call in the first line, and the type most aligned with what
/// this OS *is* was the one type it could not offer. It is also what `cap-std` binds to, which is
/// DECISIONS §84's first preference for porting real software.
///
/// # The rights it asks for, which is the whole design
///
/// Everywhere else in this module the rule is **ask for the minimum the verb needs**, because
/// over-asking is `EPERM` rather than attenuation and a PAL cannot read its own capability. A held
/// directory has no single verb to be the minimum of: it is an object, and what will be asked of it
/// later is not knowable when it is minted.
///
/// So a `Dir` asks for **everything its parent carries**, which is what `openat(dirfd, name,
/// O_DIRECTORY)` hands back on Unix and is the honest analogue. It cannot widen anything: the
/// server's answer is `parent & requested`, so the result is the projection of an authority this
/// process already holds onto one directory inside it.
///
/// Knowing what the parent carries is the awkward part, because **the contract has no way to say
/// "attenuate to whatever you have"** and no verb that reports a handle's rights. The common case
/// costs one message: ask for `dir::ALL`, and a full-rights grant answers with the handle. A
/// narrowed grant answers `EPERM`, and then [`Dir::held_rights`] finds out exactly what is there by
/// asking for one right at a time, one message per bit of `dir::ALL` (seven today), once per
/// `Dir::open` at the first hop only, since every hop after that descends from a parent whose
/// rights are now known.
///
/// That probe is a workaround and is recorded as one: the fix is a sentinel in `OPENDIR`'s rights
/// word meaning "the parent's, whatever they are", which is a contract change and therefore not a
/// lane's to make. See the BUGS section of notes/std.md.
pub struct Dir {
    /// The server's handle, `proto::ROOT` for the granted directory itself.
    pub(super) at: At,
    /// What [`Self::at`] is known to carry. [`Dir::UNKNOWN`] for the granted directory, whose
    /// rights are the endpoint's and which nothing on the wire reports.
    pub(super) rights: u64,
}

impl Dir {
    /// The rights of the granted directory: not a mask, a statement that we have not been told.
    /// `u64::MAX` cannot be confused with a real one, since `dir::ALL` is seven bits.
    const UNKNOWN: u64 = u64::MAX;

    /// The granted directory itself, which costs no message: it is what the endpoint is bound to
    /// rather than a name inside anything.
    pub(super) fn root() -> Dir {
        Dir { at: At(proto::ROOT), rights: Dir::UNKNOWN }
    }

    /// Descend one name, asking for everything this directory carries. See the type's header for
    /// why "everything it carries" is the right ask for an object and the wrong ask for a verb.
    pub(super) fn child(&self, p: &mut Page, name: &str) -> io::Result<Dir> {
        let want = if self.rights == Dir::UNKNOWN { fsproto::dir::ALL } else { self.rights };
        match descend(p, &self.at, name, want) {
            Ok(at) => Ok(Dir { at, rights: want }),
            // The one recoverable refusal: we asked for more than this capability holds, and the
            // server told us so rather than quietly handing back less (§42). Find out what is
            // actually there and ask again. Only reachable from a directory whose rights we were
            // never told, so it happens at most once per `Dir::open`.
            Err(e) if e.kind() == io::ErrorKind::PermissionDenied && self.rights == Dir::UNKNOWN => {
                let held = self.held_rights(p, name)?;
                let at = descend(p, &self.at, name, held)?;
                Ok(Dir { at, rights: held })
            }
            Err(e) => Err(e),
        }
    }

    /// Which of the rights in `dir::ALL` this directory carries, found by asking for one at a time.
    ///
    /// Each `OPENDIR` here succeeds exactly when the parent holds that one right, because the
    /// server's test is `parent & requested == requested`. One message and one close per bit, and
    /// the handles are released as they are minted rather than held, so this never approaches the
    /// contract's per-session handle budget.
    ///
    /// A parent that cannot be descended at all answers `ENOENT` rather than `EPERM` (§47 withholds
    /// a naming right by denying the name, so a holder cannot map what it may not reach), and that
    /// propagates out of the first probe as an ordinary `NotFound`.
    pub(super) fn held_rights(&self, p: &mut Page, name: &str) -> io::Result<u64> {
        let mut held = 0;
        // Every bit of `dir::ALL`, lowest first, rather than a list of names. The list this
        // replaced named six rights and `ALL` had carried a seventh (`SETTIME`, milestone 47)
        // since 2026-08-24, so a `Dir` under a narrowed grant silently dropped it. Harmless so far,
        // because no `Dir` operation asks for `SETTIME`; walking the mask is what keeps the next
        // right from being missed the same way when one does.
        let mut rest = fsproto::dir::ALL;
        while rest != 0 {
            let right = rest & rest.wrapping_neg();
            rest &= !right;
            match descend(p, &self.at, name, right) {
                // Dropping the `At` closes it: this was a question, not a capability we wanted.
                Ok(_) => held |= right,
                Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {}
                Err(e) => return Err(e),
            }
        }
        Ok(held)
    }

    /// **Open a directory.** `""` and `"."` are the granted directory and cost nothing; anything
    /// else is one descent per component, each carrying the last one's rights forward.
    ///
    /// The `OpenOptions` are upstream's and are ignored, which is the same answer Unix gives: a
    /// `dirfd` opened `O_RDONLY` still unlinks and still opens a file for writing, because what it
    /// carries is the authority over the directory rather than a mode. Here that authority is
    /// literal.
    pub fn open(path: &Path, _opts: &OpenOptions) -> io::Result<Dir> {
        if !is_reachable() {
            return Err(unsupported_err());
        }
        if count_names(path)? == 0 {
            return Ok(Dir::root());
        }
        let mut p = page();
        let mut cur = Dir::root();
        for name in names(path) {
            cur = cur.child(&mut p, name)?;
        }
        Ok(cur)
    }

    /// **Open a directory with the minimum permission for traversal** (`#![feature(dirfd)]`,
    /// rust-lang/rust#120426, added to the trait every backend implements the day this overlay
    /// was bumped past `nightly-2026-08-26`). Unix answers this with `O_PATH`/`O_SEARCH`: a
    /// handle that can be descended through but not necessarily enumerated.
    ///
    /// **This contract has no such right to ask for.** [`fsproto::dir`] has seven bits
    /// (`ENUMERATE`, `READ`, `WRITE`, `CREATE`, `REMOVE`, `DESCEND`, `SETTIME`) and [`Dir`]'s own
    /// header explains why every open here asks for all of them anyway: a held directory is an
    /// object, not a verb, and there is no wire sentinel meaning "the least you'll let me have". So
    /// the honest answer is [`Dir::open`] itself, which already asks for everything the parent
    /// carries, a superset of "enough to traverse".
    pub fn open_for_traversal(path: &Path) -> io::Result<Dir> {
        let mut opts = OpenOptions::new();
        opts.read(true);
        Dir::open(path, &opts)
    }

    /// **A second handle to the directory this one already holds**, which upstream added to every
    /// backend on `nightly-2026-09-22` (`std::fs::Dir::try_clone`, still `#![feature(dirfd)]`).
    /// Unix answers it with `dup`, Windows with `DuplicateHandle`, and both are cheap because a
    /// descriptor is a copyable reference to an object the kernel owns.
    ///
    /// **Here it is the gap [`Dir::open_dir`] and [`File::duplicate`] already record**, arrived at
    /// from a third direction: a handle is a token the FS server minted for one session, the
    /// contract has no dup verb, and copying the number would forge a second owner of the same
    /// handle, including its close. So this refuses for anything the server minted.
    ///
    /// The granted directory is the exception and costs nothing, for the reason [`Dir::root`]
    /// gives: `proto::ROOT` is a sentinel rather than a minted handle, any number of `Dir`s may
    /// hold it at once, and its `Drop` closes nothing. That is the same split `open_dir("")`
    /// already makes, so the two answers cannot disagree.
    pub fn duplicate(&self) -> io::Result<Dir> {
        if self.at.0 == proto::ROOT { Ok(Dir::root()) } else { Err(unsupported_err()) }
    }

    /// Open a file **under this directory**, which is the call the whole type exists for: the name
    /// is resolved against a capability the program is holding, not against anything ambient. A
    /// nested `path` walks from here exactly as [`walk`] walks from the granted directory.
    pub fn open_file(&self, path: &Path, opts: &OpenOptions) -> io::Result<File> {
        if !opts.read && !opts.write && !opts.append {
            return Err(io::const_error!(
                io::ErrorKind::InvalidInput,
                "an open must ask for read, write, or append"
            ));
        }
        let handle = {
            let mut p = page();
            let (at, name) = walk(&mut p, self.at.0, path, open_rights(opts))?;
            p.put(name.as_bytes());
            request(proto::req(proto::OPEN, at.0, name.len() as u64), 0)?
        };
        if opts.truncate {
            if let Err(e) = request(proto::req(proto::TRUNCATE, handle, 0), 0) {
                let _ = request(proto::req(proto::CLOSE, handle, 0), 0);
                return Err(e);
            }
        }
        let file = File {
            handle,
            pos: AtomicU64::new(0),
            append: opts.append,
            size_hint: AtomicU64::new(0),
        };
        if opts.append {
            let end = file.file_attr()?.size;
            file.pos.store(end, Ordering::Relaxed);
        }
        Ok(file)
    }

    /// What there is to know about a directory over this contract, which is that it is one. The
    /// size is the placeholder [`FileAttr`] describes: no verb reports a directory's size, and
    /// inventing one would be a fact the contract does not carry. `modified()` refuses for
    /// [`Mtime`]'s reason: a held directory is a handle, and `GETMTIME` asks by name. Its parent can
    /// answer for it (`std::fs::metadata` of the path it was opened by).
    pub fn self_metadata(&self) -> io::Result<FileAttr> {
        Ok(FileAttr { size: 0, dir: true, mtime: Mtime::Unnamed })
    }

    /// A name's metadata relative to this directory, the dirfd shape upstream std grew on
    /// 2026-09-30 (`Dir::metadata(path)` in `fs.rs` now reaches here). The same four messages
    /// [`stat`] issues, walked from this handle rather than the root: the authority is identical
    /// and only the starting point differs. An empty relative path is the directory itself, the
    /// same placeholder [`self_metadata`] returns.
    pub fn metadata(&self, path: &Path) -> io::Result<FileAttr> {
        if !is_reachable() {
            return Err(unsupported_err());
        }
        if count_names(path)? == 0 {
            return Ok(FileAttr { size: 0, dir: true, mtime: Mtime::Unnamed });
        }
        stat_under(self.at.0, path)
    }

    /// `symlink_metadata` relative to this directory. No symlink crosses this contract, so
    /// following one and not following one are the same thing, the same line [`lstat`] draws.
    pub fn symlink_metadata(&self, path: &Path) -> io::Result<FileAttr> {
        self.metadata(path)
    }

    /// `UNLINK` a name under this directory. A directory is refused (`IsADirectory`), the same line
    /// the path-shaped [`unlink`] draws.
    pub fn remove_file(&self, path: &Path) -> io::Result<()> {
        let mut p = page();
        let (at, name) = walk(&mut p, self.at.0, path, fsproto::dir::REMOVE)?;
        p.put(name.as_bytes());
        request(proto::req(proto::UNLINK, at.0, name.len() as u64), 0)?;
        Ok(())
    }

    /// **Rename across two held directories**, which is the shape `RENAME` was specified for: the
    /// source handle rides in the request word and the destination's in the second, so this is one
    /// atomic message rather than a copy and an unlink.
    pub fn rename(&self, from: &Path, to_dir: &Dir, to: &Path) -> io::Result<()> {
        let mut p = page();
        let (src_at, src) = walk(&mut p, self.at.0, from, fsproto::dir::REMOVE)?;
        let (dst_at, dst) = walk(&mut p, to_dir.at.0, to, fsproto::dir::CREATE)?;
        if src.len() + dst.len() > PAGE {
            return Err(io::const_error!(
                io::ErrorKind::InvalidFilename,
                "the two names together are longer than the page shared with the FS server"
            ));
        }
        p.put(src.as_bytes());
        p.put_at(src.len(), dst.as_bytes());
        request(
            proto::req(proto::RENAME, src_at.0, src.len() as u64),
            proto::rename_dst(dst_at.0, dst.len() as u64),
        )?;
        Ok(())
    }

    /// `MKDIR` under this directory, the `self`-relative twin of [`DirBuilder::mkdir`] (which
    /// walks from the granted root). Same verb, same close-what-you-don't-keep reasoning: the
    /// handle `MKDIR` hands back is closed immediately, since std's `create_dir` returns `()`
    /// and a later `open_dir`/`read_dir` of the same name mints another one.
    pub fn create_dir(&self, path: &Path) -> io::Result<()> {
        let mut p = page();
        let (at, name) =
            walk(&mut p, self.at.0, path, fsproto::dir::CREATE | fsproto::dir::DESCEND)?;
        p.put(name.as_bytes());
        let handle = request(proto::req(proto::MKDIR, at.0, name.len() as u64), 0)?;
        let _ = request(proto::req(proto::CLOSE, handle, 0), 0);
        Ok(())
    }

    /// **Open a directory relative to this one.** One descent per component, exactly
    /// [`Dir::open`]'s walk, just seeded from `self` instead of the granted root.
    ///
    /// `""`/`"."` is the one case that walk cannot express: it names `self` again, and this
    /// contract has no verb to mint a second handle to a directory `self` already holds (the
    /// same gap [`File::duplicate`] documents). Under the granted directory itself that costs
    /// nothing, because [`Dir::root`] is the `ROOT` sentinel and any number of `Dir`s may hold
    /// it at once (its `Drop` never closes it); anywhere else, honestly, this refuses rather
    /// than inventing a duplicate the server was never asked to mint.
    pub fn open_dir(&self, path: &Path, _opts: &OpenOptions) -> io::Result<Dir> {
        if count_names(path)? == 0 {
            return if self.at.0 == proto::ROOT { Ok(Dir::root()) } else { Err(unsupported_err()) };
        }
        let mut p = page();
        let mut names = names(path);
        let mut cur = self.child(&mut p, names.next().expect("count_names checked above"))?;
        for name in names {
            cur = cur.child(&mut p, name)?;
        }
        Ok(cur)
    }

    /// `RMDIR` under this directory, the `self`-relative twin of the path-shaped [`rmdir`].
    /// Empty-only, same as that one: `DirectoryNotEmpty` is the answer for anything else, and a
    /// file underneath is refused with `NotADirectory`.
    pub fn remove_dir(&self, path: &Path) -> io::Result<()> {
        let mut p = page();
        let (at, name) = walk(&mut p, self.at.0, path, fsproto::dir::REMOVE)?;
        p.put(name.as_bytes());
        request(proto::req(proto::RMDIR, at.0, name.len() as u64), 0)?;
        Ok(())
    }
}

impl fmt::Debug for Dir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Dir").field("handle", &self.at.0).finish()
    }
}
