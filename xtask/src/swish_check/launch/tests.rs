//! Tests of the launch helpers: reading the UART a character at a time, and the `rg` walk.

#[test]
fn a_character_split_across_two_reads_arrives_whole() {
    // `§` is C2 A7: the first read ends after C2.
    let mut split = Vec::new();
    let first = super::whole_characters(&mut split, b"BUGS \xC2");
    let second = super::whole_characters(&mut split, b"\xA7134\n");
    assert_eq!(format!("{first}{second}"), "BUGS \u{a7}134\n");
    // A byte that can never start a character is still replaced rather than held forever.
    assert_eq!(super::whole_characters(&mut split, b"\xFFx"), "\u{fffd}x");
    assert!(split.is_empty());
}

#[test]
fn the_rg_walk_line_types_what_the_disk_was_staged_for() {
    assert_eq!(
        super::RG_WALK_CD,
        format!("cd /{}", crate::disk::RIPGREP_WALK_HOME)
    );
    assert_eq!(
        super::RG_WALK_LINE,
        format!(
            "rg {} {}",
            crate::disk::RIPGREP_WALK_PATTERN,
            crate::disk::RIPGREP_CORPUS
        )
    );
    assert_eq!(
        super::RG_WALK_PREVIEW,
        format!("caps {}", super::RG_WALK_LINE)
    );
}

#[test]
fn the_rg_walk_verdict_ignores_order_and_catches_a_dropped_or_doubled_match() {
    let host = "crates/a/x.rs:// BUGS\ncrates/b/c/y.rs:## BUGS\ncrates/b/c/y.rs:## BUGS\n";
    // RedoxFS lists `b` before `a`: the same answer.
    let guest = "crates/b/c/y.rs:## BUGS\ncrates/a/x.rs:// BUGS\ncrates/b/c/y.rs:## BUGS\n";
    assert!(super::rg_walk_verdict(guest, host).is_ok());
    // A walk that never descended into `b/c` is the failure this line exists for.
    assert!(super::rg_walk_verdict("crates/a/x.rs:// BUGS\n", host).is_err());
    // One of two identical lines lost: a set without counts would pass this.
    let short = "crates/a/x.rs:// BUGS\ncrates/b/c/y.rs:## BUGS\n";
    assert!(super::rg_walk_verdict(short, host).is_err());
    // Something the host never printed, such as an error in the middle of the walk.
    let noisy = format!("{guest}rg: crates/b: permission denied\n");
    assert!(super::rg_walk_verdict(&noisy, host).is_err());
    assert!(super::rg_walk_verdict("", "").is_err());
}
