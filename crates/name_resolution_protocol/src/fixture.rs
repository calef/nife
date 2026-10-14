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
