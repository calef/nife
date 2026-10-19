//! **The regenerate-and-compare gate** (milestone 142 increment 3).
//!
//! Neither `ttf-parser` nor `ab_glyph_rasterizer` documents its output as stable, and the
//! rasteriser's raw output has already moved in a patch release. Two investigations measured it
//! bit-identical across aarch64 and x86_64 hosts, which is a property of its code that nothing
//! upstream defends. This test turns that assumption into a check: on every host that runs
//! `cargo xtask test`, the tables the terminal ships are exactly what the pinned pair makes from
//! the vendored fonts. A pin bump, a font swap or a host whose float arithmetic disagrees fails
//! here, naming the glyph and the pixel, instead of moving a pixel on a screen whose checker would
//! then fail somewhere far less legible.

#[test]
fn the_checked_in_atlas_is_what_the_generator_makes() {
    if let Some(why) = glyph_atlas_generator::check() {
        panic!(
            "{why}\n\nIf the change is deliberate, regenerate with \
             `cargo run --manifest-path tools/glyph_atlas_generator/Cargo.toml` and commit the \
             tables with the reason."
        );
    }
}
