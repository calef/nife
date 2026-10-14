//! **What a name resolver and the programs around it agree on** (§248 (the name resolver is its
//! own confined program)). Three parties share the resolver's endpoint, each with a different
//! capability to it:
//!
//! ```text
//!   spawner ──unbadged WRITE──┐          (GRANT: badge B may resolve names in zone Z)
//!                             ▼
//!   client ──WRITE, badge B──► name_resolver ──Stack──► net_stack ──► a name server
//!   (ATTACH a page, then RESOLVE)    │
//!                                    └──entropy──► the transaction id
//! ```
//!
//! - **The spawner** holds the one unbadged capability, mints a badged copy per client, and tells
//!   the resolver what each badge may resolve with [`OPERATION_GRANT`] ([`grant_messages`]). Only
//!   badge 0 is read as a grant, so a client cannot widen its own.
//! - **A client** delegates one page ([`OPERATION_ATTACH_PAGE_FRAME`]), writes a host name into it,
//!   and `CALL`s [`OPERATION_RESOLVE`]: the reply is a [`status`] ([`Outcome`]) and the TTL, and the
//!   addresses are in the page.
//! - **The resolver** keeps a [`Grants`] table and judges each name against its caller's zone
//!   before it sends anything.
//!
//! A grant is a **zone**, compared label by label (`domain_name_system::Name::is_within`): granted
//! `nife.test`, a client may ask about `packages.nife.test` and not `evilnife.test`. One host name
//! is an exact grant, and the empty zone is any name. It applies to the name asked, not to the
//! CNAME chain the answer follows.
//!
//! | bytes | field | written by |
//! |---|---|---|
//! | [`OFF_NAME`].. | the host name, dotted ASCII, at most [`NAME_TEXT_MAX`] bytes | the client |
//! | [`OFF_ADDRESSES`].. | up to [`MAX_ADDRESSES`] IPv4 addresses, four bytes each | the resolver |
//!

//! # BUGS
//!
//! - A grant lasts until the spawner sends [`OPERATION_FORGET`]; at most [`GRANTS_MAX`] at once.
//! - One zone per badge: two unrelated domains need two badges, or their common parent.
//! - Grant messages carry eight bytes each (`RECEIVE_CAP` returns two words, one the control word),
//!   so a 253-byte zone is 32 messages, once per client.
//!
//! Name: provisional (milestone 384 (in a capability system the resolver is a grant)'s lane):
//! named for what it is the protocol of. Refused `resolver_protocol` (the program, not the act)
//! and `dns_protocol` (§154 (the acronym test)).

#![no_std]

mod wire;

pub use wire::*;

/// The resolver's own copy of the bound, checked against the parser's, because `wire` is copied
/// into std's PAL where `domain_name_system` cannot be named.
const _: () = assert!(MAX_ADDRESSES == domain_name_system::MAX_ADDRESSES);

// =================================================================================================
// The resolver's endowment.
// =================================================================================================

/// **What the resolver is told at spawn**, in its three start words. Which name server to ask is
/// the spawner's decision, made from the DHCP lease (`socket_protocol::lease`) or from a
/// configuration, never the resolver's, the way the network time client is told its server.
pub mod endowment;

/// **The boot test's cases**, in one place for the two programs that must agree on them: the test
/// client asks each name in order, and the kernel's test (`system_tests/src/user/name_resolver_tests.rs`)
/// judges each reply by its index. `socket_protocol::fixture`'s split: the peer that answers,
/// `helpers/name-server-peer`, spells its zone again in Python, so the two sides are written
/// independently.
pub mod fixture;

mod grants;
pub use grants::*;

#[cfg(test)]
mod tests;

mod messages;

pub use messages::{forget_message, grant_messages, reject_code};
