//! **The disk tree: a read-only view of a whole disk as one file, served without a cache**
//! (milestone 834 (ioping on nife and Linux)). The pure half of `components/src/disk_tree.rs`.
//!
//! A `filesystem_contract::fs` namespace of exactly two names: [`DIRECTORY`] under the root, and
//! [`FILE`] in it, whose bytes are the disk's bytes from block 0 and whose size is the disk's. It
//! exists so a program run at the prompt can measure the device rather than a file server's
//! memory: `redoxfs_server` keeps whole small files and recently read blocks in memory, and Linux's
//! answer to the same problem is `/dev/nvme0n1` opened with `O_DIRECT`. Every [`fs::READ`] this
//! answers becomes exactly one device read ([`Answer::Read`]); there is no buffer here to answer
//! from, and the tests below hold that.
//!
//! Read-only by construction twice over: no mutating verb is answered (each is
//! [`dir::EROFS`]), and the server's block endpoint is badged with a window the NVMe server never
//! writes from (`non_volatile_memory_express::can_write`).
//!
//! **What the shell sees** (calef, 2026-10-18 21:36 UTC, "Yes, read-only at prompt"): this tree is
//! the shell's second mounted tree, so `cd` into [`DIRECTORY`] and name [`FILE`]:
//!
//! ```text
//! cd /disk/nvme0
//! ioping -D -c 1000 -i 0 -s 4k -S 64m -o 1m n1
//! ```
//!
//! # BUGS
//!
//! - **One file is the whole namespace**, whatever partitions are on it. A partition is a byte
//!   range of [`FILE`] (`-o` and `-S` for ioping); naming them would need the table read here.
//! - **A read longer than the server's transfer window comes back short**, at most
//!   `blk::TRANSFER_BLOCKS` blocks from the first one touched. POSIX allows it and every client in
//!   this tree loops on a short read.
//!
//! Name: provisional (lane `lane/ioping-direct-io`, 2026-10-18 UTC), for the crate, the program,
//! [`DIRECTORY`] and [`FILE`]. The mount path is `grant_plan::MOUNTED_TREE_PATH`'s.

#![no_std]

use filesystem_contract::{blk, dir, dirent, fs, operation, reply_err, verb, xattr};

/// The one directory under the root: the controller, as Linux's `nvme0`.
pub const DIRECTORY: &[u8] = b"nvme0";
/// The one file in [`DIRECTORY`]: namespace 1, as Linux's `nvme0n1`.
pub const FILE: &[u8] = b"n1";

/// What the root carries: list it, walk into it, read what is in it. Nothing that writes, so no
/// handle this server mints can carry a right to write either ([`dir::Rights::attenuate`] cannot
/// widen).
pub const ROOT_RIGHTS: u64 = dir::ENUMERATE | dir::READ | dir::DESCEND;

/// Open handles across every client, the root included. The table is per server, as
/// `redoxfs_server`'s is, because a caretaker translates its client's numbers into these.
pub const HANDLES: usize = 32;

const ENOENT: i32 = 2;
const EBADF: i32 = 9;
const EMFILE: i32 = 24;
const EOPNOTSUPP: i32 = 95;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Node {
    Root,
    Directory,
    Namespace,
}

impl Node {
    fn child(self, name: &[u8]) -> Option<Node> {
        match self {
            Node::Root if name == DIRECTORY => Some(Node::Directory),
            Node::Directory if name == FILE => Some(Node::Namespace),
            _ => None,
        }
    }

    fn entries(self) -> &'static [(&'static [u8], bool)] {
        match self {
            Node::Root => &[(DIRECTORY, true)],
            Node::Directory => &[(FILE, false)],
            Node::Namespace => &[],
        }
    }
}

/// **One device read, and what to do with it.** The server reads `blocks` blocks starting at
/// `block` into its transfer window, copies `len` bytes starting `skip` bytes in to the start of
/// the client's page, and replies `len`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Read {
    /// The first device block, in [`blk::BLOCK_SIZE`] units.
    pub block: u64,
    /// How many blocks, `1..=blk::TRANSFER_BLOCKS`.
    pub blocks: usize,
    /// Where in the first block the client's bytes start.
    pub skip: usize,
    /// How many bytes the client gets, never 0.
    pub len: usize,
}

/// What a request comes to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    /// Reply with these two words; the page already holds whatever the reply describes.
    Reply(i64, u64),
    /// Go to the device first ([`Read`]).
    Read(Read),
}

fn err(errno: i32) -> Answer {
    Answer::Reply(reply_err(errno), 0)
}

/// The namespace and its handle table.
pub struct Tree {
    size: u64,
    handles: [Option<(Node, dir::Rights)>; HANDLES],
}

impl Tree {
    /// A tree over a disk of `size` bytes, with the root installed at [`fs::ROOT`].
    pub fn new(size: u64) -> Self {
        let mut handles = [None; HANDLES];
        handles[fs::ROOT as usize] = Some((Node::Root, dir::Rights::root(ROOT_RIGHTS)));
        Tree { size, handles }
    }

    fn get(&self, handle: u64) -> Option<(Node, dir::Rights)> {
        self.handles.get(handle as usize).copied().flatten()
    }

    fn install(&mut self, entry: (Node, dir::Rights)) -> Answer {
        match self.handles.iter().position(Option::is_none) {
            Some(h) => {
                self.handles[h] = Some(entry);
                Answer::Reply(h as i64, 0)
            }
            None => err(EMFILE),
        }
    }

    /// **Answer one request.** `w0` and `w1` are the request's two words; `page` is the client's
    /// shared page, holding the name for a verb that takes one and receiving a listing for
    /// [`fs::READDIR`].
    pub fn handle(&mut self, w0: u64, w1: u64, page: &mut [u8]) -> Answer {
        let code = operation(w0);
        let Some(v) = verb::of(code) else {
            return err(dir::EINVAL);
        };
        let Some((node, rights)) = self.get(fs::req_handle(w0)) else {
            return err(EBADF);
        };
        // A copy, because a listing is written into the page the name arrived in.
        let mut name = [0u8; 256];
        let n = fs::req_len(w0);
        if v.carries_len() && code != fs::READ && code != fs::WRITE {
            if n > name.len() || n > page.len() {
                return err(dir::EINVAL);
            }
            name[..n].copy_from_slice(&page[..n]);
        }
        let name = &name[..n.min(name.len())];
        match code {
            fs::OPENDIR => self.open_directory(node, rights, name, w1),
            fs::OPEN => self.open_file(node, rights, name, w1),
            fs::READ => self.read(node, rights, n.min(page.len()), w1),
            fs::FSTAT => match node {
                Node::Namespace => Answer::Reply(self.size as i64, 0),
                _ => Answer::Reply(0, 0),
            },
            fs::CLOSE if fs::req_handle(w0) == fs::ROOT => err(dir::EINVAL),
            fs::CLOSE => {
                self.handles[fs::req_handle(w0) as usize] = None;
                Answer::Reply(0, 0)
            }
            fs::READDIR => list(node, rights, w1, page),
            // Nothing here was ever written, so there is nothing to make durable.
            fs::SYNC => Answer::Reply(0, 0),
            fs::LISTXATTR => Answer::Reply(0, 0),
            fs::GETXATTR => err(xattr::ENODATA),
            _ if v.mutates() => err(dir::EROFS),
            // GETMTIME, STATFS and the binding verbs: a disk has no modification time or free
            // count this server could truthfully give, and it enforces no bound subtree.
            _ => err(EOPNOTSUPP),
        }
    }

    /// Walk `name` from `(node, rights)`: every component but the last is a descent asking
    /// `DESCEND | step`, as [`fs::OPEN`] and [`fs::OPENDIR`] both define a path.
    fn walk(
        node: Node,
        rights: dir::Rights,
        name: &[u8],
        step: u64,
    ) -> Result<(Node, dir::Rights, &[u8]), i32> {
        let mut parts = name.split(|&b| b == b'/');
        let mut last = parts.next().unwrap_or_default();
        let (mut node, mut rights) = (node, rights);
        for next in parts {
            (node, rights) = descend(node, rights, last, dir::DESCEND | step)?;
            last = next;
        }
        if last.is_empty() || last == b"." || last == b".." {
            return Err(dir::EINVAL);
        }
        Ok((node, rights, last))
    }

    fn open_directory(
        &mut self,
        node: Node,
        rights: dir::Rights,
        name: &[u8],
        asked: u64,
    ) -> Answer {
        let found = Self::walk(node, rights, name, asked)
            .and_then(|(node, rights, last)| descend(node, rights, last, asked));
        match found {
            Ok(entry) => self.install(entry),
            Err(errno) => err(errno),
        }
    }

    fn open_file(&mut self, node: Node, rights: dir::Rights, name: &[u8], step: u64) -> Answer {
        let (parent, rights, last) = match Self::walk(node, rights, name, step) {
            Ok(found) => found,
            Err(errno) => return err(errno),
        };
        if rights.denies_all(dir::READ | dir::WRITE) {
            return err(ENOENT);
        }
        match parent.child(last) {
            None => err(ENOENT),
            Some(Node::Namespace) => {
                let file = rights.attenuate(rights.bits() & (dir::READ | dir::WRITE));
                match self.install((Node::Namespace, file)) {
                    Answer::Reply(h, _) if h >= 0 => Answer::Reply(h, self.size),
                    refused => refused,
                }
            }
            Some(_) => err(dir::EISDIR),
        }
    }

    fn read(&self, node: Node, rights: dir::Rights, asked: usize, offset: u64) -> Answer {
        match node {
            Node::Namespace if rights.allows(dir::READ) => {}
            Node::Namespace => return err(EBADF),
            _ => return err(dir::EISDIR),
        }
        if offset >= self.size || asked == 0 {
            return Answer::Reply(0, 0);
        }
        let bs = blk::BLOCK_SIZE;
        let skip = (offset % bs as u64) as usize;
        let room = blk::TRANSFER_BLOCKS * bs - skip;
        let len = asked
            .min(room)
            .min((self.size - offset).min(usize::MAX as u64) as usize);
        Answer::Read(Read {
            block: offset / bs as u64,
            blocks: (skip + len).div_ceil(bs),
            skip,
            len,
        })
    }
}

/// One step down, as [`fs::OPENDIR`] defines it: `DESCEND` on the parent or the name is not there,
/// a directory or `ENOTDIR`, and exactly the rights asked for or `EPERM`.
fn descend(
    node: Node,
    rights: dir::Rights,
    name: &[u8],
    asked: u64,
) -> Result<(Node, dir::Rights), i32> {
    if name.is_empty() || name == b"." || name == b".." {
        return Err(dir::EINVAL);
    }
    if !rights.allows(dir::DESCEND) {
        return Err(ENOENT);
    }
    match node.child(name) {
        None => Err(ENOENT),
        Some(Node::Namespace) => Err(dir::ENOTDIR),
        Some(child) => {
            let narrowed = rights.attenuate(asked);
            if narrowed.bits() != asked {
                return Err(dir::EPERM);
            }
            Ok((child, narrowed))
        }
    }
}

fn list(node: Node, rights: dir::Rights, cursor: u64, page: &mut [u8]) -> Answer {
    if node == Node::Namespace {
        return err(dir::ENOTDIR);
    }
    if !rights.allows(dir::ENUMERATE) {
        return err(dir::EPERM);
    }
    let mut at = 0;
    for &(name, is_dir) in node.entries().iter().skip(cursor as usize) {
        match dirent::encode(&mut page[at..], name, is_dir) {
            Some(n) => at += n,
            None => break,
        }
    }
    Answer::Reply(at as i64, 0)
}

#[cfg(test)]
mod tests;
