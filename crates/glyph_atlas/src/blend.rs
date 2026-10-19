//! Coverage to color, in linear light.
//!
//! Name: provisional.

use crate::SRGB_TO_LINEAR;

/// **`fg` over `bg` at `coverage` of 255, in linear light**: each `0x00RRGGBB` channel is taken
/// to linear light through [`SRGB_TO_LINEAR`], mixed there, and brought back to the sRGB code
/// whose linear value is nearest. 0 is exactly `bg` and 255 exactly `fg`.
///
/// Mixing the sRGB codes directly makes light text on a dark ground look bolder than its outline
/// and dark on light thinner, because sRGB's codes are not proportional to light; Solarized Dark
/// is the first case. Half coverage of white on black is half the light, sRGB 188, not code 128.
/// Integer only: the table is a `const`, and the way back is a search of the same table, so no
/// second table can disagree with the first.
///
/// ```
/// use glyph_atlas::blend;
///
/// assert_eq!(blend(0x00ff_ffff, 0x0000_0000, 0), 0x0000_0000);
/// assert_eq!(blend(0x00ff_ffff, 0x0000_0000, 255), 0x00ff_ffff);
/// assert_eq!(blend(0x00ff_ffff, 0x0000_0000, 128), 0x00bc_bcbc);
/// assert_eq!(blend(0x00ff_0000, 0x0000_00ff, 128), 0x00bc_00bb);
/// ```
///
/// Name: provisional.
pub const fn blend(fg: u32, bg: u32, coverage: u8) -> u32 {
    // Most of a screen is background or solid ink, and the kernel test runs this for every pixel
    // of the scanout; the two ends are also the two values the arithmetic below must return exactly.
    match coverage {
        0 => return bg,
        u8::MAX => return fg,
        _ => {}
    }
    let a = coverage as u32;
    let mut out = 0;
    let mut shift = 0;
    while shift < 24 {
        let f = SRGB_TO_LINEAR[((fg >> shift) & 0xff) as usize] as u32;
        let b = SRGB_TO_LINEAR[((bg >> shift) & 0xff) as usize] as u32;
        out |= (to_srgb((f * a + b * (255 - a) + 127) / 255) as u32) << shift;
        shift += 8;
    }
    out
}

/// The sRGB code whose linear value in [`SRGB_TO_LINEAR`] is nearest `linear`, the lower on a tie.
/// A binary search: the table rises strictly, which `the_linear_table_rises_and_round_trips`
/// holds it to.
pub(crate) const fn to_srgb(linear: u32) -> u8 {
    // The first code whose linear value is at least `linear`.
    let (mut lo, mut hi) = (0usize, 256usize);
    while lo < hi {
        let mid = (lo + hi) / 2;
        if (SRGB_TO_LINEAR[mid] as u32) < linear {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    if lo == 256 {
        return u8::MAX;
    }
    if lo > 0 && linear - SRGB_TO_LINEAR[lo - 1] as u32 <= SRGB_TO_LINEAR[lo] as u32 - linear {
        return (lo - 1) as u8;
    }
    lo as u8
}
