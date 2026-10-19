//! **The owner's grants**: the `grant` lines of a generation, read strictly (milestone 892 (a std
//! program at the prompt holds the network and a resolver); calef's ruling "a" on #2405). The line
//! format is the crate's documentation's; this module is the type and its readers, split from the
//! crate root under §266 (a Rust source file stays under 2,000 lines).
//!
//! Name: provisional, minted 2026-10-19 (UTC) by milestone/892-installed-programs-hold-the-network-and-zones.

use super::generation::{Line, classify, entries};
use super::{Digest, Error};

/// **One `grant` line**: what the owner granted the bytes with this digest. Read with
/// [`grants`] and [`network_granted`].
///
/// Name: provisional, minted 2026-10-19 (UTC) by milestone/892-installed-programs-hold-the-network-and-zones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Grant<'a> {
    /// The executable's digest, as its row has it.
    pub digest: Digest,
    /// What was granted.
    pub what: Granted<'a>,
}

/// **What a `grant` line grants.**
///
/// Name: provisional, minted 2026-10-19 (UTC) by milestone/892-installed-programs-hold-the-network-and-zones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Granted<'a> {
    /// The network: a client view of the stack, connecting out.
    Network,
    /// Names in one resolver zone, as dotted text; `""` is the root zone, written `.` in the line.
    Zone(&'a str),
}

/// The spelling of the root zone in a `grant` line, where an empty word cannot be written.
///
/// Name: provisional, minted 2026-10-19 (UTC) by milestone/892-installed-programs-hold-the-network-and-zones.
pub const ROOT_ZONE: &str = ".";

/// The rest of a `grant` line after its first word: a digest, then `network`, or `zone` and one
/// canonical zone (`.` for the root).
pub(super) fn classify_grant<'a>(mut words: impl Iterator<Item = &'a str>) -> Line<'a> {
    let Some(Ok(digest)) = words.next().map(measured_boot::parse_digest) else {
        return Line::Malformed;
    };
    let what = match (words.next(), words.next(), words.next()) {
        (Some("network"), None, _) => Granted::Network,
        (Some("zone"), Some(ROOT_ZONE), None) => Granted::Zone(""),
        (Some("zone"), Some(zone), None)
            if domain_name_resolution_contract::is_canonical_zone(zone.as_bytes()) =>
        {
            Granted::Zone(zone)
        }
        _ => return Line::Malformed,
    };
    Line::Grant(Grant { digest, what })
}

/// **Every `grant` line in a generation**, in file order, or [`Error::Malformed`]: at the first
/// line that is malformed, at a grant whose digest names no row, and at a grant that does not sort
/// strictly after the one before (one grant record has one spelling).
///
/// Name: provisional, minted 2026-10-19 (UTC) by milestone/892-installed-programs-hold-the-network-and-zones.
pub fn grants(table: &str) -> impl Iterator<Item = Result<Grant<'_>, Error>> {
    let mut previous: Option<Grant<'_>> = None;
    table
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(move |line| match classify(line) {
            Line::Grant(g) => {
                let sorted = previous.is_none_or(|p| grant_before(&p, &g));
                let named = entries(table).flatten().any(|e| e.digest == g.digest);
                previous = Some(g);
                Some(if sorted && named {
                    Ok(g)
                } else {
                    Err(Error::Malformed)
                })
            }
            Line::Row { .. } | Line::Pointer { .. } => None,
            Line::Malformed => Some(Err(Error::Malformed)),
        })
}

/// Whether `a` sorts strictly before `b` as their lines' text does.
pub(super) fn grant_before(a: &Grant<'_>, b: &Grant<'_>) -> bool {
    fn key<'g>(g: &Grant<'g>) -> ([u8; measured_boot::DIGEST_TEXT_LEN], &'static str, &'g str) {
        match g.what {
            Granted::Network => (measured_boot::digest_text(&g.digest), "network", ""),
            Granted::Zone("") => (measured_boot::digest_text(&g.digest), "zone", ROOT_ZONE),
            Granted::Zone(z) => (measured_boot::digest_text(&g.digest), "zone", z),
        }
    }
    key(a) < key(b)
}

/// **Whether the owner granted the bytes with this digest the network**: a `grant <digest>
/// network` line, read through [`grants`], so a malformed table grants nothing and says so.
///
/// Name: provisional, minted 2026-10-19 (UTC) by milestone/892-installed-programs-hold-the-network-and-zones.
pub fn network_granted(table: &str, digest: &Digest) -> Result<bool, Error> {
    let mut granted = false;
    for g in grants(table) {
        let g = g?;
        granted |= g.digest == *digest && g.what == Granted::Network;
    }
    Ok(granted)
}
