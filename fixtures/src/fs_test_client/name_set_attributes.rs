//! **The attribute witness behind a name-set grant** (milestone 61 (the caretakers: one verb table,
//! and names that say what you get)).
//!
//! Cut out of `fs_test_client.rs` unchanged on 2026-10-12 (UTC) by lane
//! maintainer/files-over-1500, for §282 (when a file is split, the parent becomes an index). What
//! another role uses is `pub(crate)` here.
//!
//! Name: provisional (lane maintainer/files-over-1500, 2026-10-12 UTC), for the module and the
//! file.

use filesystem_protocol::{dir, fs, xattr};
use filesystem_fixture as fixture;
use user_mode_runtime::{call, exit, send};

use crate::channel::{FILE, REPORT, dir_descend, dir_open, get_page, put_page, put_page_at};

/// **The attribute witness behind a name-set grant** (milestone 61, the third caretaker).
///
/// `fs_nameset_caretaker` narrows a directory capability to a designated *set* of names, and it is
/// the one caretaker that inspects a name on every request. Milestone 61 taught it four verbs whose
/// operand is an **attribute** name rather than a directory name, and that distinction is the whole
/// risk: filtering an attribute name against the set would refuse a program its own file's
/// attributes, and *not* filtering a directory name would be an escape. This asks both questions.
///
/// It is told nothing about its grant. The kernel test wires it over the `sub` fixture with a set of
/// exactly one name, so `inner` is designated and `deeper`, one entry away in the same directory, is
/// not. `rm`'s witness already asks the naming question with `REMOVE` alone; this asks it with
/// `READ`, because closing an attribute gap by opening a naming one would be a poor trade.
///
/// The attribute is removed before it reports, so the file the gate's post-run host check pins is
/// left byte for byte as it was found, with nothing attached.
pub(crate) fn set_attrs() -> ! {
    use fixture::{dirscape as esc, tree};
    let mut v = 0u64;
    let mut buf = [0u8; 128];

    // The name the set designates. Everything below hangs off this, so its failure is the control.
    let own = dir_open(fs::ROOT, tree::INNER);
    if own < 0 {
        v |= esc::GRANTED_ACCESS_FAILED;
    } else {
        v |= esc::OPENED_ITS_OWN;

        // A listing: a read, so it needs what reading the file needs and nothing more. Zero
        // attributes is a successful listing, which is what tells "forwarded and there were none"
        // apart from "the caretaker refused it".
        let (l, _) = call(FILE, fs::req(fs::LISTXATTR, own as u64, 0), 0);
        if (l as i64) < 0 {
            v |= esc::GRANTED_ACCESS_FAILED;
        } else {
            v |= esc::READ_ATTRS;
        }

        // A set, read straight back with its type code, then removed. The attribute's name is not a
        // name in the directory, so a caretaker that filtered it against the set would refuse this
        // and the bit would stay clear.
        put_page(fixture::attrs::NAME);
        put_page_at(fixture::attrs::NAME.len(), fixture::attrs::VALUE);
        let (s, _) = call(
            FILE,
            fs::req(fs::SETXATTR, own as u64, fixture::attrs::NAME.len() as u64),
            xattr::spec(fixture::attrs::KIND, fixture::attrs::VALUE.len() as u64),
        );
        if (s as i64) >= 0 {
            put_page(fixture::attrs::NAME);
            let (g, _) = call(
                FILE,
                fs::req(fs::GETXATTR, own as u64, fixture::attrs::NAME.len() as u64),
                0,
            );
            get_page(xattr::reply_value_len(g as i64), &mut buf);
            if (g as i64) >= 0
                && xattr::reply_kind(g as i64) == fixture::attrs::KIND
                && xattr::reply_value_len(g as i64) == fixture::attrs::VALUE.len()
                && buf[..fixture::attrs::VALUE.len()] == *fixture::attrs::VALUE
            {
                v |= esc::SET_AN_ATTR;
            } else {
                v |= esc::GRANTED_ACCESS_FAILED;
            }
            put_page(fixture::attrs::NAME);
            let (rm, _) = call(
                FILE,
                fs::req(
                    fs::REMOVEXATTR,
                    own as u64,
                    fixture::attrs::NAME.len() as u64,
                ),
                0,
            );
            if (rm as i64) < 0 {
                v |= esc::GRANTED_ACCESS_FAILED;
            }
        } else {
            v |= esc::GRANTED_ACCESS_FAILED;
        }
    }

    // The naming question, by both routes: a file the set does not carry, and a directory it does
    // not carry. Both are really in the granted directory and the caretaker one hop up could open
    // either on any request it liked, so each refusal is a fact about the set.
    if dir_open(fs::ROOT, tree::DEEPER) >= 0 || dir_descend(fs::ROOT, tree::DEEPER, dir::READ) >= 0
    {
        v |= esc::REACHED_AN_UNMATCHED_NAME;
    }

    send(REPORT, fixture::VERDICT, v, 0);
    exit();
}
