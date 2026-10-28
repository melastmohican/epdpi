#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Pure waveform engine tests, identical in blocking and async builds.

use epdpi::prelude::*;

type P = Ed047tc1;

#[test]
fn lut_is_16_levels_by_8_passes_of_valid_codes() {
    assert_eq!(ED047TC1_LUT.len(), 128);
    assert_eq!(P::PASSES, 8);
    assert!(ED047TC1_LUT.iter().all(|&c| c <= DriveCode::Lighten as u8));
    // Black row only darkens, white row only lightens.
    assert!(ED047TC1_LUT[..8]
        .iter()
        .all(|&c| c == DriveCode::Darken as u8));
    assert!(ED047TC1_LUT[120..]
        .iter()
        .all(|&c| c == DriveCode::Lighten as u8));
}

#[test]
fn panel_geometry_derived_constants() {
    assert_eq!(P::FB_STRIDE_BYTES, 480);
    assert_eq!(P::FB_BYTES, 480 * 540);
    assert_eq!(P::SCANLINE_PIXEL_BYTES, 240);
    assert_eq!(P::SCANLINE_BYTES, 248);
}

#[test]
fn uniform_scanline_packs_code_and_zeroes_padding() {
    let mut out = [0xEEu8; P::SCANLINE_BYTES];
    WaveformEngine::uniform_scanline::<P>(DriveCode::Darken, &mut out);
    assert!(out[..240].iter().all(|&b| b == 0x55));
    assert!(out[240..].iter().all(|&b| b == 0));
    WaveformEngine::uniform_scanline::<P>(DriveCode::Lighten, &mut out);
    assert!(out[..240].iter().all(|&b| b == 0xAA));
    WaveformEngine::uniform_scanline::<P>(DriveCode::Hold, &mut out);
    assert!(out.iter().all(|&b| b == 0));
}

#[test]
fn gray_scanline_all_white_and_all_black() {
    let mut out = [0xEEu8; P::SCANLINE_BYTES];
    let white = [0xFFu8; P::FB_STRIDE_BYTES];
    let black = [0x00u8; P::FB_STRIDE_BYTES];
    for pass in 0..P::PASSES {
        WaveformEngine::gray_scanline::<P>(&white, pass, &mut out);
        assert!(out[..240].iter().all(|&b| b == 0xAA), "white pass {pass}");
        WaveformEngine::gray_scanline::<P>(&black, pass, &mut out);
        assert!(out[..240].iter().all(|&b| b == 0x55), "black pass {pass}");
        assert!(out[240..].iter().all(|&b| b == 0));
    }
}

#[test]
fn gray_scanline_orders_pixels_left_to_right_from_bit_7() {
    // Pixels: 0 (black), 15 (white), 1, 14 -> LUT rows 0, 15, 1, 14 at pass 0.
    let mut row = [0xFFu8; P::FB_STRIDE_BYTES];
    row[0] = 0x0F;
    row[1] = 0x1E;
    let mut out = [0u8; P::SCANLINE_BYTES];
    WaveformEngine::gray_scanline::<P>(&row, 0, &mut out);
    let lut = |g: usize| ED047TC1_LUT[g * 8];
    let expected = (lut(0) << 6) | (lut(15) << 4) | (lut(1) << 2) | lut(14);
    assert_eq!(out[0], expected);
    assert_eq!(out[0], 0b01_10_10_10);
    // Pass 1 of the same pixels.
    WaveformEngine::gray_scanline::<P>(&row, 1, &mut out);
    let lut1 = |g: usize| ED047TC1_LUT[g * 8 + 1];
    assert_eq!(
        out[0],
        (lut1(0) << 6) | (lut1(15) << 4) | (lut1(1) << 2) | lut1(14)
    );
}

#[test]
fn mono_scanline_holds_unchanged_and_drives_changed() {
    let prev = [0xFFu8; P::FB_STRIDE_BYTES]; // all white
    let mut next = [0xFFu8; P::FB_STRIDE_BYTES];
    next[0] = 0x0F; // px0 white -> black, px1 stays white
    next[1] = 0xF7; // px2 stays white, px3 (7) is below the threshold -> black
    let mut out = [0xEEu8; P::SCANLINE_BYTES];
    WaveformEngine::mono_scanline::<P>(&prev, &next, &mut out);
    assert_eq!(out[0], 0b01_00_00_01);
    assert!(out[1..240].iter().all(|&b| b == 0));
    // And back: black -> white lightens.
    WaveformEngine::mono_scanline::<P>(&next, &prev, &mut out);
    assert_eq!(out[0], 0b10_00_00_10);
}

#[test]
fn pass_counts() {
    assert_eq!(WaveformEngine::passes::<P>(RefreshMode::Quality16), 8);
    assert_eq!(WaveformEngine::passes::<P>(RefreshMode::FastMono), 4);
    assert_eq!(WaveformEngine::passes::<P>(RefreshMode::Clear), 0);
}
