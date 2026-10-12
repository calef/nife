use super::*;

/// The xterm set this terminal shipped until milestone 141 replaced it, kept as the palette
/// check's known-bad witness.
const XTERM_PALETTE: [u32; 16] = [
    0x0000_0000,
    0x00cd_0000,
    0x0000_cd00,
    0x00cd_cd00,
    0x0000_00ee,
    0x00cd_00cd,
    0x0000_cdcd,
    0x00e5_e5e5,
    0x007f_7f7f,
    0x00ff_5c5c,
    0x0000_ff00,
    0x00ff_ff00,
    0x005c_5cff,
    0x00ff_00ff,
    0x0000_ffff,
    0x00ff_ffff,
];

/// Solarized Dark exactly as Schoonover publishes it, in his `16/8 TERMCOL` slots
/// (github.com/altercation/solarized, README, "The Values", read 2026-09-26).
const SOLARIZED_DARK_AS_PUBLISHED: [u32; 16] = [
    0x0007_3642,
    0x00dc_322f,
    0x0085_9900,
    0x00b5_8900,
    0x0026_8bd2,
    0x00d3_3682,
    0x002a_a198,
    0x00ee_e8d5,
    0x0000_2b36,
    0x00cb_4b16,
    0x0058_6e75,
    0x0065_7b83,
    0x0083_9496,
    0x006c_71c4,
    0x0093_a1a1,
    0x00fd_f6e3,
];

/// **The palette is Solarized Dark but for one unit of blue in entry 14**, and that unit is why
/// it passes. Published Solarized fails exactly one property on exactly one entry (`base1`,
/// `93,a1,a1`, repeats a channel); the shipped table differs from it in that entry alone, by one.
/// So a later edit that drifts another entry, or "restores" the published value, fails here
/// with the reason in front of it rather than as an unexplained compile error.
#[test]
fn the_palette_is_solarized_dark_but_for_one_unit() {
    let published = palette_faults(&SOLARIZED_DARK_AS_PUBLISHED);
    assert_eq!(
        published,
        PaletteFaults {
            repeated_channel: 1 << 14,
            ..PaletteFaults::NONE
        },
        "published Solarized Dark fails on base1 alone"
    );
    assert_eq!(palette_faults(&PALETTE), PaletteFaults::NONE);
    for i in 0..16 {
        if i == 14 {
            assert_eq!(
                SOLARIZED_DARK_AS_PUBLISHED[i] - PALETTE[i],
                1,
                "one unit of blue"
            );
        } else {
            assert_eq!(
                PALETTE[i], SOLARIZED_DARK_AS_PUBLISHED[i],
                "entry {i} is as published"
            );
        }
    }
}

/// **The palette check fails on the palette it was written about**, which is what makes its
/// silence on any other palette evidence. That palette shipped until 2026-09-26 and is kept here
/// as the check's witness. These are the numbers milestone 141 measured by hand on
/// 2026-08-19: every one of the sixteen entries repeats a channel, eight pairs are channel
/// permutations (red `cd0000` and green `00cd00` first among them), and all three channels
/// saturate in more than one entry.
#[test]
fn the_palette_check_catches_every_fault_in_the_xterm_palette() {
    let f = palette_faults(&XTERM_PALETTE);
    assert_eq!(
        f.repeated_channel, 0xffff,
        "all sixteen entries repeat a channel"
    );
    assert_eq!(f.permuted_pairs, 8, "eight pairs are channel permutations");
    assert_eq!(
        f.shared_saturation, 0b111,
        "red, green and blue each saturate twice"
    );
}

/// **Each property fires alone**, so a check that had collapsed two of them into one condition
/// (or lost one outright) cannot hide behind a palette that fails all three at once. The base
/// is sixteen colours built to pass: distinct channels, no permutations, nothing at `0xff`.
#[test]
fn each_palette_property_fires_on_its_own_fault() {
    let mut clean = [0u32; 16];
    for (i, c) in clean.iter_mut().enumerate() {
        let i = i as u32;
        *c = (0x10 + i) << 16 | (0x40 + 2 * i) << 8 | (0x90 + 3 * i);
    }
    assert_eq!(
        palette_faults(&clean),
        PaletteFaults::NONE,
        "the base must pass"
    );

    let mut repeated = clean;
    repeated[5] = 0x0033_3380;
    assert_eq!(palette_faults(&repeated).repeated_channel, 1 << 5);
    assert_eq!(palette_faults(&repeated).permuted_pairs, 0);

    let mut permuted = clean;
    // clean[1]'s channels, rotated: (r, g, b) becomes (b, r, g).
    permuted[9] = (clean[1] & 0xff) << 16 | (clean[1] >> 16) << 8 | (clean[1] >> 8 & 0xff);
    assert_eq!(palette_faults(&permuted).permuted_pairs, 1);
    assert_eq!(palette_faults(&permuted).repeated_channel, 0);

    let mut duplicated = clean;
    duplicated[15] = clean[0];
    assert_eq!(
        palette_faults(&duplicated).permuted_pairs,
        1,
        "identity is a permutation"
    );

    let mut saturated = clean;
    saturated[3] = 0x0021_ff45;
    assert_eq!(
        palette_faults(&saturated),
        PaletteFaults::NONE,
        "one entry at 0xff is allowed"
    );
    saturated[11] = 0x0031_ff55;
    assert_eq!(
        palette_faults(&saturated).shared_saturation,
        0b010,
        "green, twice"
    );
}
