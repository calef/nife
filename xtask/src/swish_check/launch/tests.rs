//! Tests of the launch helpers: reading the UART a character at a time, the `rg` walk, and reading
//! the screen back.

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

/// **The leg reads back what the guest's terminal draws.** A `Vt` at the scanout's own grid is
/// painted pixel for pixel into a screendump, exactly as `Vt::pixel` fills the surface, and
/// `screen_rows` must find the typed line in it. The decoder matches the atlas's anti-aliased
/// cells exactly, so a decoder still reading the 7x8 bitmap, or a font the guest does not draw
/// with, reads `?`s here instead of on a QEMU boot.
#[test]
fn the_screen_is_read_back_in_the_font_the_terminal_draws() {
    let mut vt =
        video_terminal::Vt::new(video_terminal::script::COLS, video_terminal::script::ROWS);
    vt.feed(b"nife\r\n$ ls 42");
    let (w, h) = (graphics_contract::WIDTH, graphics_contract::HEIGHT);
    let mut ppm = format!("P6\n{w} {h}\n255\n").into_bytes();
    for y in 0..h {
        for x in 0..w {
            let p = vt.pixel(x, y);
            ppm.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, p as u8]);
        }
    }
    let alphabet: Vec<u8> = (b'a'..=b'z').chain(b'0'..=b'9').chain(*b" $").collect();
    let screen = super::screen_rows(&ppm, &alphabet).expect("a screendump of the scanout's size");
    assert_eq!(screen.rows[0].trim_end(), "nife");
    // The cursor block sits on the cell after the last character, drawn reversed, so it matches
    // nothing; everything before it reads.
    assert_eq!(&screen.rows[1][..7], "$ ls 42");
}
