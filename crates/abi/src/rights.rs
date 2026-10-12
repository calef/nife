/// Read the object's contents.
pub const READ: u64 = 1 << 0;
/// Modify the object's contents.
pub const WRITE: u64 = 1 << 1;
/// Delegate a (possibly narrowed) copy of this capability to another capability table.
pub const GRANT: u64 = 1 << 2;

/// **Learn what exists, without acting on it** (milestone 126). The kernel-level twin of
/// `filesystem_protocol`'s directory `ENUMERATE`; `capability::Rights::ENUMERATE` carries the full
/// argument and the list of objects expected to grow it.
pub const ENUMERATE: u64 = 1 << 3;

/// **Add a thread to a process** (milestone 812 (`std::thread::spawn` runs real threads in one
/// address space), §269 (how threads share a process) fork 1 as ruled on pull request #1892).
/// `capability::Rights::BIND` carries the argument; see [`super::process`].
pub const BIND: u64 = 1 << 4;
