//! **What a name may be, and how a walk under a subtree grant meets this server**: the single
//! component rule, and the glue between RedoxFS nodes and errors and `subtree_scope`.
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

use super::*;

/// What a RedoxFS node is, in the terms `subtree_scope` decides with. RedoxFS has no mount points.
pub(crate) fn kind_of(node: &Node) -> Kind {
    if node.is_dir() {
        Kind::Directory
    } else if node.is_symlink() {
        Kind::Symlink
    } else {
        Kind::File
    }
}

/// A `subtree_scope` refusal as this crate's error.
pub(crate) fn refused(r: subtree_scope::Refusal) -> Error {
    Error::new(r.errno())
}

/// An engine error carried through `subtree_scope::walk`'s lookup, so a device error on the way
/// down is reported as itself rather than as a refusal. It exists because neither the engine's
/// error type nor the scope crate's is this crate's, so no `From` between them may be written.
pub(crate) struct Fault(pub(crate) Error);

impl From<subtree_scope::Refusal> for Fault {
    fn from(r: subtree_scope::Refusal) -> Self {
        Fault(refused(r))
    }
}

/// **A name the server owns: copied out of where it arrived, before anything judges it** (milestone
/// 825 (a hostile client races the file server's name window), calef's "Refused names" ruling,
/// 2026-10-15 UTC). Every verb that resolves a name takes one, and the only constructor,
/// [`NameCopy::copy_in`], copies the bytes into this value's own bounded buffer and validates
/// UTF-8 on the copy. Nothing builds one from a borrowed `&str`, so a name read straight from a
/// client's shared window cannot reach the resolve: that is a type error, not a review comment.
///
/// The text is reachable only through [`NameCopy::component`] and [`NameCopy::path`], which run
/// `check_component` on the copy first, so a verb cannot resolve a name it has not checked either.
/// The check runs where each verb has always run it (after `dir_at` for the path verbs), which
/// keeps every verb's error order as it was; that is why the check is not in the constructor.
///
/// A window-borrowed `&str` does not reach a verb:
///
/// ```compile_fail,E0308
/// # fn reach<D: redoxfs::Disk>(srv: &mut redoxfs_server::Server<D>, window: &[u8]) {
/// let name = core::str::from_utf8(window).unwrap();
/// let _ = srv.open_file_at(0, name);
/// # }
/// ```
///
/// A copy does:
///
/// ```
/// # fn reach<D: redoxfs::Disk>(
/// #     srv: &mut redoxfs_server::Server<D>,
/// #     window: &[u8],
/// # ) -> redoxfs_server::engine_error::Result<u32> {
/// let name = redoxfs_server::NameCopy::copy_in(window)?;
/// srv.open_file_at(0, &name)
/// # }
/// ```
///
/// Name: provisional (lane milestone/825, 2026-10-15 UTC); an architect names types.
pub struct NameCopy {
    bytes: [u8; NAME_MAX],
    len: usize,
}

/// The longest name a [`NameCopy`] holds: one block, the length `Server::handle` clamps every
/// name to, so a copy from the window never truncates.
const NAME_MAX: usize = filesystem_protocol::blk::BLOCK_SIZE;

impl NameCopy {
    /// Copy `src` into a new value and validate UTF-8 on the copy. `EINVAL` for invalid UTF-8 or a
    /// name longer than one block, the answers a malformed name has always had.
    pub fn copy_in(src: &[u8]) -> Result<Self> {
        if src.len() > NAME_MAX {
            return Err(Error::new(EINVAL));
        }
        let mut bytes = [0u8; NAME_MAX];
        bytes[..src.len()].copy_from_slice(src);
        core::str::from_utf8(&bytes[..src.len()]).map_err(|_| Error::new(EINVAL))?;
        Ok(Self {
            bytes,
            len: src.len(),
        })
    }

    /// The copy as text. `copy_in` validated it and the buffer is never written again, so the
    /// empty fallback cannot happen; if it ever did, the empty name fails every check closed.
    fn text(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or_default()
    }

    /// The copy as one checked component: `check_component`'s rule, on the server's bytes.
    pub(crate) fn component(&self) -> Result<&str> {
        let name = self.text();
        check_component(name)?;
        Ok(name)
    }

    /// The copy as a checked relative path: every `/`-separated component passes
    /// `check_component`, so `..` and empty steps are `EINVAL` whatever the tree holds.
    pub(crate) fn path(&self) -> Result<&str> {
        let path = self.text();
        for step in path.split('/') {
            check_component(step)?;
        }
        Ok(path)
    }
}

/// Reject anything that is not a single path component, with `EINVAL`.
///
/// **This is our rule, enforced at our boundary, and it was previously true only by accident.** The
/// contract has always said a name is a single component resolved under the bound directory, with no
/// absolute path and no `..` escape (§27, and this module's own header). RedoxFS does not enforce that:
/// its `check_name` rejects only `:`, over-long names, and duplicates, so `/`, `.` and `..` pass
/// straight through. Nothing walked paths, so nothing escaped, which made the invariant hold by the
/// absence of a walker rather than by a check.
///
/// Adding `CREATE` (milestone 31 phase 2) turned that from a latent oddity into something a client
/// could *write*: `create_file("../escape")` created a directory entry literally named `../escape` in
/// the bound directory. Still not a traversal, and still a landmine, because the moment anything walks
/// paths (a per-directory grant binding a subtree, the host tool, or the image mounted through Redox's
/// FUSE driver) that entry means something it should never have been able to mean. A found-and-fixed
/// gap, not a theoretical one: the test that caught it asserted a property the docs already claimed.
///
/// Deliberately checked here rather than patched into the vendored engine: it is a rule of *our*
/// contract, not a bug in RedoxFS, whose callers are free to name entries whatever they like.
/// **And the attribute store's name is reserved, everywhere** (milestone 57). A store a client could
/// name would be part of the namespace, and then "the attributes of a file" would be reachable as
/// ordinary bytes by anything holding the directory they live in, which is precisely what DECISIONS
/// §34's amendment says to get right. It is refused in every directory rather than only in the root
/// so that the rule a client has to remember is one sentence instead of a location-dependent one,
/// and `EINVAL` is the answer because the name is not expressible here rather than not permitted.
/// [`Server::read_dir`] is the other half: what cannot be named is also not listed.
pub(crate) fn check_component(name: &str) -> Result<()> {
    // What a name is anywhere is `subtree_scope`'s (ruling D); what RedoxFS adds is this server's.
    if !subtree_scope::is_name(name.as_bytes())
        || name == xattr::STORE_DIR
        || name.contains('\\')
        || name.contains(':')
    {
        return Err(Error::new(EINVAL));
    }
    Ok(())
}
