/// `invoke(reply_cap, REPLY, r0, r1, _)` -> 0. Deliver `{r0, r1}` to the caller, wake it, and
/// consume this capability (a second use is [`crate::Error::NoSuchSlot`]).
pub const REPLY: u64 = 0;

/// `invoke(reply, REPLY_CAPABILITY, r0, r1, capability_slot)` -> 0, or 1 when the copy did not
/// reach the caller. [`REPLY`], carrying one capability:
/// a copy of the capability in the server's `capability_slot` is filed in the caller's table, and the
/// caller's `CALL` returns the slot it landed in as `x2` (§255 (each socket is its own
/// capability); name ratified by calef 2026-10-07 (UTC), number provisional, milestone 649 (every client of a network stack shares its
/// socket numbers)).
///
/// - **The carried capability needs `GRANT`**, exactly as [`super::rendezvous::SEND_CAP`]'s
///   delegated capability does, and keeps the rights it has in the server's table. There is no
///   word for a rights mask, so a server narrows first: the network stack mints each socket from
///   a `WRITE | GRANT` copy of its own endpoint, and the copy it answers with has those rights.
/// - **A refused source consumes nothing.** `NoSuchSlot` or `NotPermitted` comes back, nothing
///   reaches the caller, and this Reply is still held and can still answer with [`REPLY`].
/// - **A full table drops the copy, not the answer.** The caller gets `r0`, `r1` and
///   [`super::rendezvous::NO_CAP`] in `x2`, the same thing a server sees when a `CALL`'s Reply
///   did not fit its table, and the server's call returns 1 rather than 0, so it can undo what
///   it minted the capability for. A caller no longer waiting is answered the same way.
/// - **No opt-in on the caller's side.** A caller asked for a reply by calling, and can delete
///   what it is given; seL4 gates reply transfer on the server's `Grant` alone. §246 (a plain
///   `RECEIVE` never takes a capability) refused filling the table of a receiver that never
///   asked, which a `CALL` caller is not. Whether a caller should be able to refuse is left open
///   in §255.
///
/// Since this method exists, **every `CALL` returns `x2`**: the delivered slot after a
/// `REPLY_CAPABILITY`, [`super::rendezvous::NO_CAP`] after a plain [`REPLY`]. Before it, `x2` came
/// back holding whatever the caller left there.
pub const REPLY_CAPABILITY: u64 = 1;
