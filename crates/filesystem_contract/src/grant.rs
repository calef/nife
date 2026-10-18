/// The granted directions, packed into the caretaker's spec word. Read alone is the common case
/// (`run wc report.txt`); write implies read, since a writer that cannot read back is a shape
/// nothing has asked for.
pub const READ: u64 = 1 << 0;
/// Implies [`READ`]: a writer that cannot read back is a shape nothing has asked for.
pub const WRITE: u64 = 1 << 1;

/// The longest granted name, in bytes. The name rides in **two `START` argument words** rather
/// than a frame, so a per-file grant costs no extra page and no extra mapping, and the
/// caretaker needs nothing mapped before it runs. Sixteen bytes is short, and deliberately so:
/// it is a demonstrator's limit, not a filesystem's, and lifting it means giving the caretaker
/// a frame, which is a change to the wiring and not to this contract.
pub const MAX_NAME: usize = 16;

/// `EROFS`, the reply to a write through a read-only grant. Chosen over `EACCES` on purpose:
/// `EACCES` is the Unix answer, "you were denied", which implies a policy that could have said
/// yes. There is no policy here. The capability carries one direction, so the honest statement
/// is about the thing itself, and `std::io::ErrorKind::ReadOnlyFilesystem` is what a caller sees.
pub const EROFS: i32 = 30;

/// `ENOTDIR`, the reply to a [`super::fs::CREATE`] through a file grant. "This is a file, not a
/// directory" is a fact about what the holder has, not a refusal of what it asked.
pub const ENOTDIR: i32 = 20;

/// The handle the caretaker mints for the one file it serves. Fixed, because there is exactly
/// one: a holder that guesses a different number gets `EBADF` from the same check every other
/// handle goes through.
pub const HANDLE: u64 = 0;

/// Pack a granted name into the two argument words the caretaker is started with. Names shorter
/// than [`MAX_NAME`] are zero-padded; longer ones are refused by the caller (see [`fits`]).
pub const fn pack_name(name: &[u8]) -> (u64, u64) {
    let mut lo = [0u8; 8];
    let mut hi = [0u8; 8];
    let mut i = 0;
    while i < name.len() && i < 8 {
        lo[i] = name[i];
        i += 1;
    }
    while i < name.len() && i < MAX_NAME {
        hi[i - 8] = name[i];
        i += 1;
    }
    (u64::from_le_bytes(lo), u64::from_le_bytes(hi))
}

/// Unpack a granted name into `buf` (at least [`MAX_NAME`] bytes) and return its length.
pub fn unpack_name(lo: u64, hi: u64, len: usize, buf: &mut [u8; MAX_NAME]) -> usize {
    buf[..8].copy_from_slice(&lo.to_le_bytes());
    buf[8..].copy_from_slice(&hi.to_le_bytes());
    len.min(MAX_NAME)
}

/// Whether a name can travel as a per-file grant at all.
pub const fn fits(name: &[u8]) -> bool {
    !name.is_empty() && name.len() <= MAX_NAME
}

/// Pack the name length and the granted rights into the caretaker's third argument word.
///
/// # Examples
///
/// `wc` starts holding one file, read-only, with the name in two argument words.
///
/// ```
/// use filesystem_contract::grant;
///
/// assert!(grant::fits(b"report.txt"));
/// let (lo, hi) = grant::pack_name(b"report.txt");
/// let spec = grant::spec(b"report.txt".len(), grant::READ);
///
/// // What the caretaker unpacks from its three argument words.
/// let mut buf = [0u8; grant::MAX_NAME];
/// let n = grant::unpack_name(lo, hi, grant::spec_len(spec), &mut buf);
/// assert_eq!(&buf[..n], b"report.txt");
///
/// // A read-only grant is not a policy that could have said yes, so a write gets EROFS.
/// assert!(!grant::is_writable(spec));
/// assert_eq!(grant::EROFS, 30);
///
/// // A name too long to travel in two words is refused by the caller, not truncated on the wire.
/// assert!(!grant::fits(b"a-name-longer-than-sixteen-bytes"));
/// ```
pub const fn spec(len: usize, rights: u64) -> u64 {
    ((len as u64) & 0xff) | (rights << 8)
}

/// The granted name's length from a spec word.
pub const fn spec_len(w: u64) -> usize {
    (w & 0xff) as usize
}

/// The granted rights from a spec word.
pub const fn spec_rights(w: u64) -> u64 {
    w >> 8
}

/// Whether a spec word grants writing.
pub const fn is_writable(w: u64) -> bool {
    spec_rights(w) & WRITE != 0
}

/// **A grant whose operand is the holder's whole namespace**, spelled as a name of zero length.
///
/// A per-file or per-directory grant names one thing, and [`fits`] refuses an empty name, so a
/// length of zero cannot mean a name and is free to mean something else. It means the grant
/// carries a **set** ([`super::nameset`]), which does not fit in two argument words, so the
/// program is told "act on everything you can see" and reads the set by enumerating the
/// capability it was handed. That is not a loophole: the namespace of a set capability *is* the
/// set, so enumerating it reveals exactly what the command line already showed.
pub const WHOLE_NAMESPACE: usize = 0;

#[cfg(test)]
mod tests;
