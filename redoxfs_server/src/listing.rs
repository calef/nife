//! **A directory's listing as `READDIR` sends it**, and the encode that writes it from a cursor.
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

use super::*;

/// **A directory's listing as `READDIR` sends it**: every child but the attribute store, sorted by
/// name so a cursor means the same thing across calls, each with its node and whether it is a
/// directory.
///
/// # BUGS
///
/// - **Every child costs a node read**, because the record carries
///   [`filesystem_protocol::dirent::IS_DIR`] and a RedoxFS directory entry does not store the
///   child's type: `read_tree` of each child is how this learns it. Measured by milestone 121
///   (`ripgrep` on nife: enumeration as a capability) on 2026-09-26: listing 128 files cost 43 to
///   85 us per entry under HVF. Since milestone 606 the listing is memoized ([`Memo`]), so the
///   cost is paid once per directory until something changes, not once per listing.
pub(crate) fn list<D: Disk>(
    tx: &mut Transaction<D>,
    dir: TreePtr<Node>,
) -> Result<Vec<memo::Child>> {
    let mut children = Vec::new();
    tx.child_nodes(dir, &mut children)?;
    // **The attribute store is not part of the namespace** (milestone 57 (extended attributes)).
    // Filtered here, before
    // any cursor is applied, so the cursor still counts what the client can see; a filter applied
    // after `skip` would make one entry vanish at whichever offset the store happened to sort to.
    // `check_component` refuses the name on the way in, so this is the other half of the same rule
    // rather than a second one.
    let mut out = Vec::with_capacity(children.len());
    for entry in &children {
        let Some(name) = entry.name() else { continue };
        if name == xattr::STORE_DIR {
            continue;
        }
        let child: redoxfs::TreeData<Node> = tx.read_tree(entry.node_ptr())?;
        out.push(memo::Child {
            name: name.into(),
            ptr: entry.node_ptr(),
            kind: kind_of(child.data()),
        });
    }
    out.sort_unstable_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Fill `out` with `listing`'s records from `cursor` on, and return the bytes used.
pub(crate) fn encode_listing(listing: &[memo::Child], cursor: u32, out: &mut [u8]) -> usize {
    let mut used = 0;
    for child in listing.iter().skip(cursor as usize) {
        match filesystem_protocol::dirent::encode(
            &mut out[used..],
            child.name.as_bytes(),
            child.kind == Kind::Directory,
        ) {
            Some(n) => used += n,
            // A record is never split: stop before one that will not fit and let the caller's next
            // cursor start here. That is what makes "the buffer filled" and "the directory ended"
            // distinguishable rather than a guess.
            None => break,
        }
    }
    used
}
