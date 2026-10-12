//! **What a name resolver and the programs around it agree on** (milestone 384 (in a capability
//! system the resolver is a grant); §248 (the name resolver is its own confined program)).
//!
//! Three parties share one endpoint, the resolver's, and each holds a different capability to it:
//!
//! ```text
//!   spawner ──unbadged WRITE──┐          (GRANT: badge B may resolve names in zone Z)
//!                             ▼
//!   client ──WRITE, badge B──► name_resolver ──Stack──► net_stack ──► a name server
//!   (ATTACH a page, then RESOLVE)    │
//!                                    └──entropy──► the transaction id
//! ```
//!
//! - **The spawner** holds the one unbadged capability. It mints a badged copy for each client
//!   (`abi::rendezvous::BADGE`), and tells the resolver what that badge may resolve with
//!   [`OPERATION_GRANT`] messages ([`grant_messages`] builds them). Only a message arriving with
//!   badge 0 is read as a grant, so a client cannot widen its own.
//! - **A client** holds a badged copy and nothing else of the resolver. It delegates one page
//!   ([`OPERATION_ATTACH_PAGE_FRAME`], a `SEND_CAP`), writes a host name into it, and `CALL`s
//!   [`OPERATION_RESOLVE`] with the name's length. The reply word is a [`status`] (with the address
//!   count and a detail byte, [`Outcome`]), the second word is the TTL, and the addresses are in
//!   the page at [`OFF_ADDRESSES`].
//! - **The resolver** keeps a [`Grants`] table, judges each name against the zone its caller's
//!   badge was granted **before it sends anything**, and asks the name server only about a name
//!   that passes.
//!
//! # What a grant means
//!
//! A grant is a **zone**: the zone's own name and every name under it, compared label by label
//! (`domain_name_system::Name::is_within`), so a client granted `nife.test` may resolve
//! `packages.nife.test` and may not resolve `evilnife.test`. A zone of one host name is an exact-name
//! grant in effect, and the root zone (an empty zone text) is "any name", for the client that truly
//! needs it. Those are the three options of Fork 2 in `notes/name-resolution.md`, expressed as one
//! mechanism rather than three.
//!
//! **The check applies to the name asked, not to the chain the answer follows.** A client granted
//! `packages.example.org` whose name is a CNAME into a CDN still gets the CDN's address, because the
//! answer is `domain_name_system::Query::accept`'s business and the grant is about what the client
//! may ask. What the grant stops is a client asking about a name outside its zone at all.
//!
//! # The page
//!
//! | bytes | field | written by |
//! |---|---|---|
//! | [`OFF_NAME`].. | the host name, dotted ASCII, at most [`NAME_TEXT_MAX`] bytes | the client |
//! | [`OFF_ADDRESSES`].. | up to [`MAX_ADDRESSES`] IPv4 addresses, four bytes each | the resolver |
//!
//! # EXAMPLES
//!
//! A spawner grants badge 7 the zone `nife.test`, and the resolver's table then answers for it:
//!
//! ```
//! use domain_name_system::Name;
//! use name_resolution_protocol::{Denied, Grants, grant_messages};
//!
//! let mut grants = Grants::new();
//! for (w0, w1) in grant_messages(7, b"nife.test").unwrap() {
//!     grants.control(w0, w1);
//! }
//! let asked = Name::from_dotted("packages.nife.test").unwrap();
//! assert!(grants.check(7, &asked).is_ok());
//! let elsewhere = Name::from_dotted("evilnife.test").unwrap();
//! assert_eq!(grants.check(7, &elsewhere), Err(Denied::OutsideZone));
//! assert_eq!(grants.check(8, &asked), Err(Denied::NoGrant));
//! ```
//!
//! # BUGS
//!
//! - **A grant lasts until the spawner forgets it** ([`OPERATION_FORGET`], milestone 892 (a std
//!   program at the prompt holds the network and a resolver)). At most [`GRANTS_MAX`] clients hold
//!   a grant at once; a spawner that never forgets serves only that many in the resolver's life,
//!   which is what this line said before the verb existed.
//! - **One zone per badge.** A client that needs two unrelated domains needs two badges, or a
//!   grant of their common parent, which may be the root.
//! - **Grant messages carry eight bytes each**, because the resolver receives with `RECEIVE_CAP`
//!   (its clients `CALL` and `SEND_CAP`), which returns two data words, and the first is the control
//!   word. A 253-byte zone is 32 messages. It is the spawner's one-time cost per client.
//!
//! Name: provisional 2026-10-06 (UTC), milestone 384's lane. Spelled the way
//! `network_time_protocol` and `domain_name_system` are, by what it is the protocol of. Refused
//! `resolver_protocol`, which names the program rather than the act, and `dns_protocol` under §154
//! (the acronym test): the act is name resolution, and the wire format underneath is
//! `domain_name_system`'s.

#![no_std]

mod wire;

pub use wire::*;

/// The resolver's own copy of the bound, checked against the parser's, because `wire` is copied
/// into std's PAL where `domain_name_system` cannot be named.
const _: () = assert!(MAX_ADDRESSES == domain_name_system::MAX_ADDRESSES);

/// The [`OPERATION_GRANT`] messages that grant `badge` the zone `zone` (dotted text, a trailing dot
/// allowed; empty for the root, which is every name). `None` when the badge is 0, which is the
/// spawner's own, or the text is longer than [`NAME_TEXT_MAX`] plus a trailing dot.
pub fn grant_messages(badge: u32, zone: &[u8]) -> Option<impl Iterator<Item = (u64, u64)> + '_> {
    if badge == 0 || zone.len() > NAME_TEXT_MAX + 1 {
        return None;
    }
    let total = zone.len() as u8;
    // `max(1)` so the root (no bytes) is still one message: a zero-length grant is a grant.
    let messages = zone.len().div_ceil(8).max(1);
    Some((0..messages).map(move |i| {
        let start = i * 8;
        let part = zone.get(start..).unwrap_or(&[]);
        let n = part.len().min(8);
        let mut bytes = [0u8; 8];
        bytes[..n].copy_from_slice(&part[..n]);
        (
            control_word(OPERATION_GRANT, start as u8, total, badge),
            u64::from_le_bytes(bytes),
        )
    }))
}

/// The [`OPERATION_FORGET`] message that withdraws `badge`'s grant: the control word, and an unused
/// second word. `None` for badge 0, the spawner's own, which holds no grant to withdraw.
pub const fn forget_message(badge: u32) -> Option<(u64, u64)> {
    if badge == 0 {
        return None;
    }
    Some((control_word(OPERATION_FORGET, 0, 0, badge), 0))
}

/// A stable small number for each `domain_name_system::Reject`, the detail of a
/// [`status::REFUSED`]: which check refused the reply, so that a forged answer and a broken server
/// are not one word.
pub const fn reject_code(reject: domain_name_system::Reject) -> u8 {
    use domain_name_system::Reject;
    match reject {
        Reject::Malformed(_) => 1,
        Reject::NotAResponse => 2,
        Reject::IdMismatch => 3,
        Reject::Truncated => 4,
        Reject::QuestionMismatch => 5,
        Reject::NoSuchName => 6,
        Reject::ServerError(_) => 7,
        Reject::ChainTooLong => 8,
        Reject::NoAddress => 9,
    }
}

// =================================================================================================
// The resolver's endowment.
// =================================================================================================

/// **What the resolver is told at spawn**, in its three start words. Which name server to ask is
/// the spawner's decision, made from the DHCP lease (`socket_protocol::lease`) or from a
/// configuration, never the resolver's, the way the network time client is told its server.
pub mod endowment {
    /// Ask over UDP, and again over TCP when the reply comes back truncated (RFC 1035 and RFC 7766's
    /// ordinary behavior).
    pub const TRANSPORT_UDP: u64 = 0;
    /// Ask over TCP only (RFC 7766 lets a stub resolver do this). For a name server that does not
    /// answer UDP at all, which is the QEMU runners' peer: slirp forwards only TCP to a host process.
    pub const TRANSPORT_TCP: u64 = 1;

    /// The first start word: the name server's IPv4 address, big-endian in the low 32 bits, the
    /// format `socket_protocol::lease::ipv4_word` reports a lease's name server in.
    pub const fn server(octets: [u8; 4]) -> u64 {
        u32::from_be_bytes(octets) as u64
    }

    /// The first start word back to an address.
    pub const fn server_octets(word: u64) -> [u8; 4] {
        (word as u32).to_be_bytes()
    }

    /// The second start word: the name server's port and the transport.
    pub const fn port_and_transport(port: u16, transport: u64) -> u64 {
        port as u64 | transport << 16
    }

    /// The second start word's `(port, transport)`.
    pub const fn port_and_transport_fields(word: u64) -> (u16, u64) {
        (word as u16, word >> 16)
    }

    /// **Slot 4: the resolver's own address space, `WRITE`**, so a forget can `UNMAP` the page the
    /// forgotten badge attached (`abi::address_space::UNMAP`), as `net_stack` gives up a closed
    /// socket's page through `socket_protocol::stack_slots::OWN_SPACE`. Slots 0 to 3 are the
    /// resolver's own business (`components/src/name_resolver.rs`); this one is here because the
    /// progenitor and the kernel's test harness both place it. Provisional (milestone 892).
    pub const OWN_SPACE_SLOT: u64 = 4;
}

/// **The boot test's cases**, in one place for the two programs that must agree on them: the test
/// client asks each name in order, and the kernel's test (`system_tests/src/user/name_resolver_tests.rs`)
/// judges each reply by its index. `socket_protocol::fixture`'s split: the peer that answers,
/// `helpers/name-server-peer`, spells its zone again in Python, so the two sides are written
/// independently.
pub mod fixture {
    use domain_name_system::{Error, Reject};

    use super::{reject_code, status};

    /// The zone the test grants its client.
    pub const ZONE: &str = "nife.test";
    /// The runners' name server peer, a `guestfwd` to `helpers/name-server-peer` on DNS's port, and
    /// the address its zone answers with.
    pub const SERVER: [u8; 4] = [10, 0, 2, 9];

    /// The test client's `arg0` when it was given a stack and ends by connecting to what it
    /// resolved.
    pub const WITH_NETWORK: u64 = 1;
    /// The test client's last report carries one of these: the echo through the resolved address
    /// came back,
    pub const ECHO_OK: u64 = 1;
    /// it was given no stack,
    pub const ECHO_SKIPPED: u64 = 2;
    /// the first name did not resolve, so there was nowhere to connect,
    pub const ECHO_NO_ADDRESS: u64 = 3;
    /// or the connection or the echo failed.
    pub const ECHO_FAILED: u64 = 4;

    /// One name, and the reply the client must get for it.
    pub struct Case {
        /// The host name the client writes in its page.
        pub name: &'static str,
        /// The status the reply must carry.
        pub status: u8,
        /// Its detail byte.
        pub detail: u8,
    }

    const fn case(name: &'static str, status: u8, detail: u8) -> Case {
        Case {
            name,
            status,
            detail,
        }
    }

    /// The cases, in the order the client asks them. The first resolves, and is the address the
    /// client then connects to.
    pub const CASES: [Case; 10] = [
        case("packages.nife.test", status::OK, 0),
        // A compressed CNAME to the name above, the way a recursive server answers.
        case("mirror.nife.test", status::OK, 0),
        case("nosuch.nife.test", status::NO_SUCH_NAME, 0),
        // The peer's catch-all answer is REFUSED, response code 5.
        case("unlisted.nife.test", status::SERVER_ERROR, 5),
        // The right answer under the wrong transaction id.
        case(
            "forged.nife.test",
            status::REFUSED,
            reject_code(Reject::IdMismatch),
        ),
        // An address for bank.nife.test, and none for the name asked.
        case(
            "poisoned.nife.test",
            status::REFUSED,
            reject_code(Reject::NoAddress),
        ),
        // An owner that is a compression pointer to itself.
        case(
            "loop.nife.test",
            status::REFUSED,
            reject_code(Reject::Malformed(Error::PointerForward)),
        ),
        // Outside the zone, so never asked.
        case("example.com", status::DENIED, 0),
        // Ends in the zone's text and is not under it.
        case("evilnife.test", status::DENIED, 0),
        // Not a host name at all.
        case("two words.nife.test", status::BAD_NAME, 0),
    ];
}

mod grants;
pub use grants::*;

#[cfg(test)]
mod tests;
