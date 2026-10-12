//! **Ending a TCP socket or a listener on `CLOSE`** (`net_stack`'s `Table::close`).
//!
//! Name: provisional, milestone 514 (the HVF std listener hangs in some full runs)'s lane,
//! 2026-10-12 (UTC). Moved out of `net_stack.rs` when the listener half grew, because §282 (when a
//! file is split, the parent becomes an index) gives a split parent's new code nowhere to land but a
//! child named for it. `tcp_close` names the operation; `listener_reset` was considered and names
//! only half of it.

use super::*;

/// Close the TCP socket at `handle`: a connection drains its close handshake, and a listener
/// resets any peer whose handshake already landed in it.
///
/// **A listener closed with a handshake in it resets that peer** (milestone 514, 2026-10-12 UTC).
/// `tcp_accept` re-arms before it returns, so a peer that connects between the last `ACCEPT` and
/// this `CLOSE` completes its handshake into a socket nobody will accept. Closing that socket and
/// dropping it unpolled sent nothing, so the peer held an established connection to nobody: the
/// inbound prober waited on one for the rest of the run while the next listener's `accept` timed
/// out with nobody connecting. Forced on HVF by polling 300 ms before this close, the run failed in
/// exactly that shape; with the reset the peer reconnects and the next listener serves it. Linux
/// resets a closed listener's unaccepted connections for the same reason.
///
/// Name: provisional, milestone 514's lane, 2026-10-12 (UTC). Named for its module.
pub(super) fn close_tcp(
    iface: &mut Interface,
    dev: &mut Nic,
    sockets: &mut SocketSet,
    handle: SocketHandle,
    listener: bool,
) {
    let socket = sockets.get_mut::<tcp::Socket>(handle);
    if listener {
        if !matches!(socket.state(), tcp::State::Listen | tcp::State::Closed) {
            socket.abort();
            iface.poll(instant(), dev, sockets);
        } else {
            socket.close();
        }
        return;
    }
    socket.close();
    // Drain the close handshake so the peer (and slirp's flow for it) sees a clean teardown before
    // we drop the socket. Dropping a socket mid-close leaves the peer's connection half-open, which
    // on the guestfwd echo peer blocks the next connection to it. Both FINs exchanged (Closed or
    // TimeWait) is enough; waiting for full Closed would linger in TimeWait's timer. Bounded, so a
    // peer that never finishes still returns.
    service_until(iface, dev, sockets, |s| {
        matches!(
            s.get_mut::<tcp::Socket>(handle).state(),
            tcp::State::Closed | tcp::State::TimeWait
        )
    });
}
