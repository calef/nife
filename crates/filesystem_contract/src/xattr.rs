/// The longest attribute **name**, in bytes. 255, which is Linux's `XATTR_NAME_MAX`, chosen so
/// that nothing Samba writes is refused for its name: `user.org.netatalk.Metadata` and
/// `user.com.apple.metadata:_kMDItemUserTags` are both well inside it, and a limit that refused
/// a real client's name would be discovered on the board rather than here. It is also exactly
/// what one length byte holds, which is what makes [`list`]'s records a byte cheaper than
/// [`super::dirent`]'s.
pub const MAX_NAME: usize = 255;

/// The longest attribute **value**, in bytes.
///
/// Three kibibytes, and the bound that sets it is the transport: [`fs::SETXATTR`] carries a name
/// and a value in one [`super::PAGE`], so `MAX_NAME + MAX_VALUE` must fit 4096 with room that a
/// reader can see is room. It is far above what the application needs (Apple's `AFP_AfpInfo` is
/// 60 bytes, a `FinderInfo` blob is 32) and far below what Samba's `streams_xattr` can be asked
/// to hold, because that feature stores whole alternate data streams as attributes.
///
/// **Corrected 2026-08-18.** This said `AFP_AfpInfo` was 402 bytes, which conflated two
/// different blobs. `AFP_INFO_SIZE` is `0x3c`, 60, in Samba's `source3/include/MacExtensions.h`,
/// and Apple's own SMB client carries it as `uint8_t afpinfo[60]`. The 402 is the
/// AppleDouble-shaped buffer on Samba's *netatalk* metadata path, `uint8_t ad_data[402]` in
/// `vfs_fruit.c`. The sizing argument above survives the correction with more room, not less;
/// see §99 for what does and does not fit here.
///
/// **BUGS.** A resource fork larger than this is refused with [`E2BIG`], loudly, per DECISIONS
/// §42: an attribute store that silently truncated a value would hand back a file that looked
/// intact and was not. Lifting the limit means chunking the transfer across requests, which the
/// contract does not do for anything today.
pub const MAX_VALUE: usize = 3072;

/// The most attributes one file or directory may carry.
///
/// Sixteen, and it is the number that makes [`fs::LISTXATTR`] complete in one reply: sixteen
/// names of [`MAX_NAME`] bytes plus their length prefixes is 4096 bytes, exactly one
/// [`super::PAGE`]. A cursor would otherwise be needed, and a cursored listing can be observed
/// half-changed, which is a worse property than a ceiling. Samba with `fruit` uses a handful per
/// file.
///
/// **BUGS.** The seventeenth attribute is [`ENOSPC`], not an eviction.
pub const MAX_COUNT: usize = 16;

/// The largest **kind** code, [`i32::MAX`] as a `u32`. See this module's header: the kind rides
/// in a reply word whose sign bit means "error", so bit 31 is not available to it.
pub const MAX_KIND: u32 = i32::MAX as u32;

/// The kind a POSIX-style client writes: an uninterpreted byte string. Zero, so a client that
/// knows nothing about kinds writes the right one by writing nothing.
pub const RAW: u32 = 0;

/// `ENODATA`: the named attribute is not set on this node. The only error that means absence,
/// which is the discipline `ENOENT` earns in [`dir::explain`]: an instrument that reads any
/// failure as "not there" reports filesystems that never existed (notes/fs-server.md).
pub const ENODATA: i32 = 61;
/// `ERANGE`: the name is empty, longer than [`MAX_NAME`], or holds a NUL. A fact about the name,
/// not about the capability.
pub const ERANGE: i32 = 34;
/// `E2BIG`: the value is longer than [`MAX_VALUE`].
pub const E2BIG: i32 = 7;
/// `ENOSPC`: the node already holds [`MAX_COUNT`] attributes and this would be a new one.
pub const ENOSPC: i32 = 28;
/// `EOPNOTSUPP`: **this capability does not offer extended attributes at all.**
///
/// DECISIONS §42's rule is that a backend which cannot meet a verb's guarantee does not offer
/// the verb, and that the refusal is loud. The caretakers (`fs_file_caretaker`,
/// `fs_subtree_caretaker`, `fs_nameset_caretaker`) answer this: they serve this same protocol
/// over a narrowed namespace and do not forward attribute requests, so a program behind one
/// learns that its capability does not carry attributes rather than that its request was
/// malformed. `EINVAL` would have been the silent degradation, because it reads as "you sent
/// nonsense" rather than "this does not do that".
pub const ENOTSUP: i32 = 95;

/// **The reserved name the attribute store lives under**, in the image root.
///
/// It is refused by every name-taking verb and skipped by [`fs::READDIR`], in **every**
/// directory rather than only in the root, so the rule a client has to remember is one sentence
/// instead of a location-dependent one. A store a client could name would be part of the
/// namespace, and then "the attributes of a file" would be reachable as ordinary bytes by
/// anything holding the directory.
///
/// **BUGS.** You cannot create a file called `.nife-attrs` anywhere on a nife
/// filesystem, at any rights. The refusal is `EINVAL`, the same one `..` gets, because the name
/// is not expressible rather than not permitted.
///
/// Recovery **does** see it. `redoxfs_host extract` and upstream's FUSE mount show this
/// directory as ordinary data, so a backup carries the attributes out with the files. That is a
/// decision and not a leak (notes/host-recovery.md); the store is unnameable through the
/// *contract*, and the contract is what confines a client.
pub const STORE_DIR: &str = ".nife-attrs";

/// Pack [`fs::SETXATTR`]'s second word: the value's type code in the high 32 bits, its length in
/// the low 32. The name's length rides in the request word's own length field, as every other
/// verb's payload length does, so the two lengths never share a field.
pub const fn spec(kind: u32, value_len: u64) -> u64 {
    ((kind as u64) << 32) | (value_len & 0xffff_ffff)
}
/// The type code from a [`fs::SETXATTR`] second word.
pub const fn spec_kind(w1: u64) -> u32 {
    (w1 >> 32) as u32
}
/// The value length from a [`fs::SETXATTR`] second word.
pub const fn spec_value_len(w1: u64) -> usize {
    (w1 & 0xffff_ffff) as usize
}

/// Pack [`fs::GETXATTR`]'s reply: the kind in the high bits, the value's length in the low 32.
///
/// **It is an [`i64`] and it must stay non-negative**, because this whole protocol's error
/// boundary is "negative is a negated errno" ([`super::reply_err`]). That is what costs the kind
/// its 32nd bit, and it is the right trade: a separate reply word does not exist, and a header
/// in front of the value would make every client copy the value twice.
pub const fn reply(kind: u32, value_len: usize) -> i64 {
    (((kind & MAX_KIND) as i64) << 32) | (value_len as i64 & 0xffff_ffff)
}
/// The kind of a [`fs::GETXATTR`] reply. Only meaningful when the reply is non-negative.
pub const fn reply_kind(r0: i64) -> u32 {
    ((r0 >> 32) as u32) & MAX_KIND
}
/// The value's length, in bytes, of a [`fs::GETXATTR`] reply.
pub const fn reply_value_len(r0: i64) -> usize {
    (r0 as u64 & 0xffff_ffff) as usize
}

/// Whether `name` is a name this store can hold: non-empty, at most [`MAX_NAME`] bytes, and free
/// of NUL.
///
/// **Bytes, not UTF-8, and no namespace prefix is required.** Linux insists on
/// `user.`/`security.`/`trusted.` because those namespaces mean different privilege; there is no
/// privilege here to mean, so requiring the prefix would be inventing a policy to enforce. NUL is
/// the one byte refused, because every client that will ever write one of these names (Samba, and
/// anything speaking the POSIX API) hands it over as a C string, and a name with a NUL in it
/// would come back shorter than it went out.
pub const fn is_valid_name(name: &[u8]) -> bool {
    if name.is_empty() || name.len() > MAX_NAME {
        return false;
    }
    let mut i = 0;
    while i < name.len() {
        if name[i] == 0 {
            return false;
        }
        i += 1;
    }
    true
}

/// **How [`fs::LISTXATTR`] packs names into the shared page**: one length byte, then that many
/// bytes of name, repeated. No flags byte and no type code, because a listing answers "what is
/// attached" and [`fs::GETXATTR`] answers "what is it"; carrying the kind here would double the
/// record header to save a round trip nobody has asked for.
///
/// The reply's `r0` is how many bytes were filled, so a client iterates until it has consumed
/// exactly that many, exactly as [`super::dirent`] works.
pub mod list {
    /// Bytes one record takes: one for the length, plus the name.
    pub const fn record_len(name_len: usize) -> usize {
        1 + name_len
    }

    /// The most bytes a complete listing can occupy: every attribute a node may hold, at the
    /// longest name each. Equal to [`super::super::PAGE`], which is what lets
    /// [`super::super::fs::LISTXATTR`] answer without a cursor.
    pub const MAX_BYTES: usize = super::MAX_COUNT * record_len(super::MAX_NAME);

    /// Write one record at the start of `out`, returning its length, or `None` if it does not
    /// fit or the name is not one this store holds.
    pub fn encode(out: &mut [u8], name: &[u8]) -> Option<usize> {
        let n = record_len(name.len());
        if !super::is_valid_name(name) || out.len() < n {
            return None;
        }
        out[0] = name.len() as u8;
        out[1..n].copy_from_slice(name);
        Some(n)
    }

    /// Walk the records in `buf` (exactly the `r0` bytes a [`super::super::fs::LISTXATTR`] reply
    /// filled). Stops at a zero length byte and at the first record that runs off the end, which
    /// is what a truncated reply looks like and must not be read as a name.
    pub fn iter(buf: &[u8]) -> Names<'_> {
        Names { buf, at: 0 }
    }

    /// The iterator [`iter`] returns.
    pub struct Names<'a> {
        buf: &'a [u8],
        at: usize,
    }

    impl<'a> Iterator for Names<'a> {
        type Item = &'a [u8];

        fn next(&mut self) -> Option<Self::Item> {
            let len = *self.buf.get(self.at)? as usize;
            if len == 0 {
                return None;
            }
            let name = self.buf.get(self.at + 1..self.at + 1 + len)?;
            self.at += record_len(len);
            Some(name)
        }
    }
}

/// **The store's on-disk record format, and the whole of the layer's semantics as pure
/// functions.**
///
/// One node's attributes are one blob: a sequence of records, each a [`store::RECORD_HEADER`]-byte
/// header (a name length, a `u32` kind, a `u16` value length, all little-endian) followed by the
/// name and then the value. The FS server keeps one such blob per node in a file named for the
/// node's `TreePtr` id ([`store::file_name`]) inside [`STORE_DIR`].
///
/// **Why the format is published here rather than hidden in the server.** Two reasons, and the
/// first is the honest one: every interesting rule of this feature (what replaces what, when a
/// limit is hit, what a truncated blob means) lives in [`store::set`], [`store::remove`] and [`store::get`], which are
/// pure functions over a byte slice and therefore host-tested in milliseconds instead of under an
/// emulator. The second is recovery: a person holding a damaged image and the `redoxfs_host`
/// tool can read the store, because the format is written down next to the code that makes it.
///
/// **Keyed on the node, which is what makes rename free.** A rename changes a directory entry and
/// not the node, so attributes follow a file across one without any code knowing a rename
/// happened. AppleDouble sidecars get exactly this wrong, and so would any path-keyed store; the
/// correctness is only available inside the FS server, which is an argument *for* the layer
/// rather than a consolation for it.
pub mod store {
    use super::{E2BIG, ENODATA, ENOSPC, ERANGE, MAX_COUNT, MAX_KIND, MAX_VALUE, is_valid_name};

    /// A record's fixed header: one byte of name length, four of kind, two of value length.
    pub const RECORD_HEADER: usize = 1 + 4 + 2;

    /// Bytes one record occupies.
    pub const fn record_len(name_len: usize, value_len: usize) -> usize {
        RECORD_HEADER + name_len + value_len
    }

    /// The largest a node's whole blob can be: [`MAX_COUNT`] records at the widest each. This is
    /// the ceiling on what one node costs the store, and the reason the count limit exists at
    /// all rather than being left to the disk.
    pub const MAX_BYTES: usize = MAX_COUNT * record_len(super::MAX_NAME, MAX_VALUE);

    /// The name of the file that holds a node's blob: its `TreePtr` id in lowercase hex, always
    /// eight characters, so the store directory's entries sort and read predictably in a
    /// recovery listing.
    pub const fn file_name(node_id: u32) -> [u8; 8] {
        let mut out = [b'0'; 8];
        let mut i = 0;
        while i < 8 {
            let nibble = ((node_id >> (28 - 4 * i)) & 0xf) as u8;
            out[i] = if nibble < 10 {
                b'0' + nibble
            } else {
                b'a' + nibble - 10
            };
            i += 1;
        }
        out
    }

    /// Walk a blob's records: `(name, kind, value)` each.
    ///
    /// Stops at a zero name length and at the first record that runs off the end. Both are what a
    /// **truncated or zero-padded** blob looks like, and neither may be read as an attribute: a
    /// torn write that left half a record behind must produce fewer attributes, never a value
    /// with somebody else's bytes on the end of it.
    pub fn iter(blob: &[u8]) -> Records<'_> {
        Records { blob, at: 0 }
    }

    /// How many attributes a blob holds.
    pub fn count(blob: &[u8]) -> usize {
        iter(blob).count()
    }

    /// The value and kind of one attribute, or `None` if it is not set.
    pub fn get<'a>(blob: &'a [u8], name: &[u8]) -> Option<(u32, &'a [u8])> {
        iter(blob)
            .find(|(n, ..)| *n == name)
            .map(|(_, k, v)| (k, v))
    }

    /// Write the [`super::list`] records for every attribute in `blob` into `out`, returning the
    /// bytes used. A complete listing always fits [`super::list::MAX_BYTES`]; a shorter `out`
    /// stops before the record that will not fit rather than splitting one.
    pub fn list(blob: &[u8], out: &mut [u8]) -> usize {
        let mut used = 0;
        for (name, ..) in iter(blob) {
            match super::list::encode(&mut out[used..], name) {
                Some(n) => used += n,
                None => break,
            }
        }
        used
    }

    /// **Set `name` to `value`, producing the new blob in `out` and returning its length.**
    ///
    /// An existing attribute is replaced **in place**, so the order a client sees from
    /// [`super::super::fs::LISTXATTR`] does not shuffle when a value changes; a new one is
    /// appended. Create and replace are one operation, for the reason
    /// [`super::super::fs::SETXATTR`] gives.
    ///
    /// `out` must hold `blob.len() + record_len(name.len(), value.len())`, which is always
    /// enough because replacing can only shrink the result. A shorter one is [`ENOSPC`], which a
    /// correctly sized caller never sees.
    ///
    /// Errors are errnos rather than a bespoke type, because the FS server's rule is that
    /// nothing below the serve loop speaks the wire's vocabulary and everything speaks the
    /// engine's.
    pub fn set(
        blob: &[u8],
        name: &[u8],
        kind: u32,
        value: &[u8],
        out: &mut [u8],
    ) -> Result<usize, i32> {
        if !is_valid_name(name) {
            return Err(ERANGE);
        }
        if value.len() > MAX_VALUE {
            return Err(E2BIG);
        }
        if kind > MAX_KIND {
            return Err(syscall_einval());
        }
        let mut used = 0;
        let mut replaced = false;
        for (n, k, v) in iter(blob) {
            let (n, k, v) = if n == name {
                replaced = true;
                (name, kind, value)
            } else {
                (n, k, v)
            };
            used += write_record(&mut out[used..], n, k, v).ok_or(ENOSPC)?;
        }
        if !replaced {
            if count(blob) >= MAX_COUNT {
                return Err(ENOSPC);
            }
            used += write_record(&mut out[used..], name, kind, value).ok_or(ENOSPC)?;
        }
        Ok(used)
    }

    /// **Remove `name`, producing the new blob in `out` and returning its length.** [`ENODATA`]
    /// if it was not set: the caller asked about one specific thing, so "it was not there" is the
    /// answer rather than a silent success. `out` must be at least `blob.len()`.
    pub fn remove(blob: &[u8], name: &[u8], out: &mut [u8]) -> Result<usize, i32> {
        if !is_valid_name(name) {
            return Err(ERANGE);
        }
        let mut used = 0;
        let mut found = false;
        for (n, k, v) in iter(blob) {
            if n == name {
                found = true;
                continue;
            }
            used += write_record(&mut out[used..], n, k, v).ok_or(ENOSPC)?;
        }
        if found { Ok(used) } else { Err(ENODATA) }
    }

    /// `EINVAL`, spelled here so this module needs no dependency on the error crate. The kind
    /// field is the only thing that answers it: a code with bit 31 set cannot ride home in a
    /// reply word, and truncating it would hand back a type that was never written.
    const fn syscall_einval() -> i32 {
        22
    }

    /// Lay one record at the start of `out`; `None` if it does not fit.
    fn write_record(out: &mut [u8], name: &[u8], kind: u32, value: &[u8]) -> Option<usize> {
        let n = record_len(name.len(), value.len());
        if out.len() < n || name.len() > super::MAX_NAME || value.len() > MAX_VALUE {
            return None;
        }
        out[0] = name.len() as u8;
        out[1..5].copy_from_slice(&kind.to_le_bytes());
        out[5..7].copy_from_slice(&(value.len() as u16).to_le_bytes());
        out[RECORD_HEADER..RECORD_HEADER + name.len()].copy_from_slice(name);
        out[RECORD_HEADER + name.len()..n].copy_from_slice(value);
        Some(n)
    }

    /// The iterator [`iter`] returns.
    pub struct Records<'a> {
        blob: &'a [u8],
        at: usize,
    }

    impl<'a> Iterator for Records<'a> {
        type Item = (&'a [u8], u32, &'a [u8]);

        fn next(&mut self) -> Option<Self::Item> {
            let head = self.blob.get(self.at..self.at + RECORD_HEADER)?;
            let name_len = head[0] as usize;
            if name_len == 0 {
                return None;
            }
            let kind = u32::from_le_bytes([head[1], head[2], head[3], head[4]]);
            let value_len = u16::from_le_bytes([head[5], head[6]]) as usize;
            let body = self.at + RECORD_HEADER;
            let name = self.blob.get(body..body + name_len)?;
            let value = self
                .blob
                .get(body + name_len..body + name_len + value_len)?;
            self.at += record_len(name_len, value_len);
            Some((name, kind, value))
        }
    }
}

use super::{dir, fs};

/// A [`fs::SETXATTR`] carries a name and a value in one page, so the two limits together must
/// leave the transport room. Checked at compile time because both sides are constants: a change
/// that broke it should fail the build rather than wait for a request that happens to be wide.
const _: () = assert!(MAX_NAME + MAX_VALUE <= super::PAGE);
/// And a complete listing must fit one page, which is what lets [`fs::LISTXATTR`] have no cursor.
const _: () = assert!(list::MAX_BYTES <= super::PAGE);
/// The four verbs must not collide with the thirteen that were already on the wire.
const _: () = assert!(fs::GETXATTR > fs::RMDIR && fs::REMOVEXATTR <= 0xff);
/// The rights these verbs consult are the ones [`fs::READ`] and [`fs::WRITE`] consult; nothing
/// here invents a rung, so a mask this module needs is a mask the ladder already defines.
const _: () = assert!((dir::READ | dir::WRITE) & !dir::ALL == 0);

#[cfg(test)]
mod tests;
