use redoxfs::DiskMemory;

use super::*;

mod directory_capability;
mod entries;
mod extended_attributes;

mod block_writes;
mod handles;
mod support;
mod sync_and_time;

pub(crate) use support::VecIo;
use support::{image_with, image_with_tree, list};

/// Build a 16 MiB RedoxFS image in memory with the given files in its root (using the std
/// creation APIs the host tool uses), then reopen it through a fresh [`Server`], exactly as the
/// running system does: the host tool makes the image, the server only ever opens it.
fn server_with(files: &[(&str, &[u8])]) -> Server<DiskMemory> {
    Server::open(image_with(files)).expect("open")
}

fn server_with_tree() -> Server<DiskMemory> {
    Server::open(image_with_tree()).expect("open")
}
