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
    assert!(
        ED047TC1_LUT[..8]
            .iter()
            .all(|&c| c == DriveCode::Darken as u8)
    );
    assert!(
        ED047TC1_LUT[120..]
            .iter()
            .all(|&c| c == DriveCode::Lighten as u8)
    );
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
    assert_eq!(WaveformEngine::passes::<P>(RefreshMode::M5Quality), 2 + 15);
    assert_eq!(WaveformEngine::passes::<P>(RefreshMode::M5Text), 2 + 12);
    assert_eq!(WaveformEngine::passes::<P>(RefreshMode::M5Fast), 8);
    assert_eq!(WaveformEngine::passes::<P>(RefreshMode::M5Fastest), 5);
}

#[test]
fn m5_tables_are_16_levels_of_valid_codes() {
    assert_eq!(P::ERASER_PASSES, 2);
    assert_eq!(P::QUALITY_PASSES, 15);
    assert_eq!(M5_ERASER_LUT.len(), 16 * 2);
    assert_eq!(M5_QUALITY_LUT.len(), 16 * 15);
    assert!(M5_ERASER_LUT.iter().all(|&c| c <= 3));
    assert!(M5_QUALITY_LUT.iter().all(|&c| c <= 3));
    // Spot checks against M5GFX `lut_eraser` and `lut_quality` (2 white, 1 black, 3 hold):
    // level 0 is lightened twice by the eraser, level 15 darkened twice.
    assert_eq!(&M5_ERASER_LUT[..2], &[2, 2]);
    assert_eq!(&M5_ERASER_LUT[30..], &[1, 1]);
    // Quality level 15 ends with two no-operation codes (3, 3 in M5GFX, kept as 3).
    assert_eq!(&M5_QUALITY_LUT[16 * 15 - 2..], &[3, 3]);
    // Quality level 0 starts black, white, white.
    assert_eq!(&M5_QUALITY_LUT[..3], &[1, 2, 2]);
}

#[test]
fn lut_scanline_indexes_the_table_by_the_row_levels() {
    // Eraser pass 0 of a row holding level 0 in the first pixel, 15 in the second.
    let mut out = vec![0xAAu8; P::SCANLINE_BYTES];
    let row = vec![0x0Fu8; P::FB_STRIDE_BYTES];
    WaveformEngine::lut_scanline::<P>(&row, P::ERASER_LUT, P::ERASER_PASSES, 0, &mut out);
    // Level 0 lightens (2), level 15 darkens (1), first pixel in bits 7:6.
    assert_eq!(out[0], 0b10_01_10_01);
    assert!(out[P::SCANLINE_PIXEL_BYTES..].iter().all(|&b| b == 0));
}

#[test]
fn m5_text_fast_fastest_tables_match_m5gfx() {
    assert_eq!(M5_TEXT_LUT.len(), 16 * P::TEXT_PASSES);
    assert_eq!(M5_FAST_LUT.len(), 16 * P::FAST_PASSES);
    assert_eq!(M5_FASTEST_LUT.len(), 16 * P::FASTEST_PASSES);
    for table in [&M5_TEXT_LUT[..], &M5_FAST_LUT[..], &M5_FASTEST_LUT[..]] {
        assert!(table.iter().all(|&c| c <= 3));
    }
    // `lut_fast`: black targets (level 0) go white twice then black six times, white targets
    // (level 15) the other way round. The other levels never move.
    assert_eq!(&M5_FAST_LUT[..8], &[2, 2, 1, 1, 1, 1, 1, 1]);
    assert_eq!(&M5_FAST_LUT[15 * 8..], &[1, 1, 2, 2, 2, 2, 2, 2]);
    assert!(M5_FAST_LUT[8..15 * 8].iter().all(|&c| c == 3));
    // `lut_fastest`: one kick, four drives.
    assert_eq!(&M5_FASTEST_LUT[..5], &[2, 1, 1, 1, 1]);
    assert_eq!(&M5_FASTEST_LUT[15 * 5..], &[1, 2, 2, 2, 2]);
    // `lut_text`: level 0 starts with four lightens, level 15 ends on a lighten (spot checks
    // of the transposition).
    assert_eq!(&M5_TEXT_LUT[..4], &[2, 2, 2, 2]);
    assert_eq!(M5_TEXT_LUT[15 * 12 + 11], 2);
}

fn pair_rows(prev: u8, next: u8) -> (Vec<u8>, Vec<u8>) {
    (
        vec![prev; P::FB_STRIDE_BYTES],
        vec![next; P::FB_STRIDE_BYTES],
    )
}

#[test]
fn m5_fast_holds_unchanged_pairs_and_drives_changed_ones() {
    let mut out = vec![0xAAu8; P::SCANLINE_BYTES];
    // Black stays black: every pass holds.
    let (prev, next) = pair_rows(0x00, 0x00);
    for pass in 0..P::FAST_PASSES {
        WaveformEngine::m5_scanline::<P>(RefreshMode::M5Fast, &prev, &next, pass, &mut out);
        assert!(out.iter().all(|&b| b == 0), "pass {pass}");
    }
    // Gray 9 and 15 are both white after reduction, so also unchanged.
    let (prev, next) = pair_rows(0x9F, 0xFF);
    WaveformEngine::m5_scanline::<P>(RefreshMode::M5Fast, &prev, &next, 0, &mut out);
    assert!(out[..P::SCANLINE_PIXEL_BYTES].iter().all(|&b| b == 0));
    // Black to white: the white row of `lut_fast`, 1 1 2 2 2 2 2 2.
    let (prev, next) = pair_rows(0x00, 0xFF);
    for (pass, code) in [1u8, 1, 2, 2, 2, 2, 2, 2].into_iter().enumerate() {
        WaveformEngine::m5_scanline::<P>(RefreshMode::M5Fast, &prev, &next, pass, &mut out);
        assert!(
            out[..P::SCANLINE_PIXEL_BYTES]
                .iter()
                .all(|&b| b == code * 0x55),
            "pass {pass}"
        );
    }
    assert!(out[P::SCANLINE_PIXEL_BYTES..].iter().all(|&b| b == 0));
}

#[test]
fn m5_text_leaves_white_alone_and_erases_then_draws_the_rest() {
    let mut out = vec![0xAAu8; P::SCANLINE_BYTES];
    // White before and after: nothing moves in any of the 14 passes.
    let (prev, next) = pair_rows(0xFF, 0xFF);
    for pass in 0..2 + P::TEXT_PASSES {
        WaveformEngine::m5_scanline::<P>(RefreshMode::M5Text, &prev, &next, pass, &mut out);
        assert!(out.iter().all(|&b| b == 0), "pass {pass}");
    }
    // Black on screen, white requested: the pair is not white before, so it is driven. The
    // erase follows the black on screen (both lighten), the draw follows the white target.
    let (prev, next) = pair_rows(0x00, 0xFF);
    WaveformEngine::m5_scanline::<P>(RefreshMode::M5Text, &prev, &next, 0, &mut out);
    assert!(out[..P::SCANLINE_PIXEL_BYTES].iter().all(|&b| b == 0xAA));
    let code = M5_TEXT_LUT[15 * P::TEXT_PASSES];
    WaveformEngine::m5_scanline::<P>(RefreshMode::M5Text, &prev, &next, 2, &mut out);
    assert!(
        out[..P::SCANLINE_PIXEL_BYTES]
            .iter()
            .all(|&b| b == code * 0x55)
    );
    // White on screen, black requested: driven too.
    let (prev, next) = pair_rows(0xFF, 0x00);
    WaveformEngine::m5_scanline::<P>(RefreshMode::M5Text, &prev, &next, 0, &mut out);
    assert!(out[..P::SCANLINE_PIXEL_BYTES].iter().all(|&b| b == 0x55));
}

#[test]
fn m5_scanline_holds_for_other_modes() {
    let mut out = vec![0xAAu8; P::SCANLINE_BYTES];
    let (prev, next) = pair_rows(0x00, 0xFF);
    WaveformEngine::m5_scanline::<P>(RefreshMode::M5Quality, &prev, &next, 0, &mut out);
    assert!(out.iter().all(|&b| b == 0));
}

#[test]
fn m5_scanline_drives_only_the_changed_pairs_of_a_mixed_row() {
    let mut prev = vec![0xFFu8; P::FB_STRIDE_BYTES];
    let mut next = prev.clone();
    // Pair 3 (bytes 3) goes white to black, everything else stays.
    next[3] = 0x00;
    let mut out = vec![0xAAu8; P::SCANLINE_BYTES];
    for mode in [
        RefreshMode::M5Fast,
        RefreshMode::M5Fastest,
        RefreshMode::M5Text,
    ] {
        let (erase, lut, passes) = match mode {
            RefreshMode::M5Fast => (0, &M5_FAST_LUT[..], P::FAST_PASSES),
            RefreshMode::M5Fastest => (0, &M5_FASTEST_LUT[..], P::FASTEST_PASSES),
            _ => (P::ERASER_PASSES, &M5_TEXT_LUT[..], P::TEXT_PASSES),
        };
        for pass in 0..erase + passes {
            WaveformEngine::m5_scanline::<P>(mode, &prev, &next, pass, &mut out);
            // Byte 3 of the framebuffer row is pixels 6 and 7, bus byte 1 holds pixels 4 to 7:
            // pixel 6 is bits 3:2, pixel 7 bits 1:0, both black targets from white.
            let code = |gray: usize, step: usize| {
                if pass < erase {
                    M5_ERASER_LUT[15 * P::ERASER_PASSES + pass]
                } else {
                    lut[gray * passes + step]
                }
            };
            let draw = code(0, pass.saturating_sub(erase));
            let expected = (draw << 2) | draw;
            for (i, &b) in out[..P::SCANLINE_PIXEL_BYTES].iter().enumerate() {
                let want = if i == 1 { expected } else { 0 };
                assert_eq!(b, want, "{mode:?} pass {pass} byte {i}");
            }
        }
    }
    // Swapping the rows (black to white) must also drive that pair.
    std::mem::swap(&mut prev, &mut next);
    WaveformEngine::m5_scanline::<P>(RefreshMode::M5Fast, &prev, &next, 0, &mut out);
    assert_ne!(out[1], 0);
}

#[test]
fn rect_aligns_to_pixel_pairs_and_clips_to_the_panel() {
    // Odd left edge rounds down, odd right edge rounds up.
    assert_eq!(
        Rect::new(3, 5, 4, 2).aligned::<P>(),
        Some(Rect::new(2, 5, 6, 2))
    );
    // Already aligned stays.
    assert_eq!(
        Rect::new(4, 0, 8, 1).aligned::<P>(),
        Some(Rect::new(4, 0, 8, 1))
    );
    // Clipped to the panel.
    assert_eq!(
        Rect::new(950, 530, 100, 100).aligned::<P>(),
        Some(Rect::new(950, 530, 10, 10))
    );
    // Nothing left.
    assert_eq!(Rect::new(0, 0, 0, 5).aligned::<P>(), None);
    assert_eq!(Rect::new(0, 0, 5, 0).aligned::<P>(), None);
    assert_eq!(Rect::new(960, 0, 5, 5).aligned::<P>(), None);
    assert_eq!(Rect::new(0, 540, 5, 5).aligned::<P>(), None);
    // Huge sizes do not overflow.
    assert_eq!(
        Rect::new(1, 1, u32::MAX, u32::MAX).aligned::<P>(),
        Some(Rect::new(0, 1, 960, 539))
    );
}

#[test]
fn rect_union_grows_and_ignores_empty() {
    let a = Rect::new(10, 20, 5, 5);
    let b = Rect::new(30, 10, 5, 5);
    assert_eq!(a.union(b), Rect::new(10, 10, 25, 15));
    assert_eq!(a.union(Rect::new(0, 0, 0, 0)), a);
    assert_eq!(Rect::new(0, 0, 0, 9).union(b), b);
    assert_eq!(Rect::full::<P>(), Rect::new(0, 0, 960, 540));
}

#[test]
fn mask_scanline_keeps_only_the_pixels_inside_the_columns() {
    // Pixels 2..6: the low half of byte 0 and the high half of byte 1.
    let mut out = vec![0xFFu8; P::SCANLINE_PIXEL_BYTES];
    out.resize(P::SCANLINE_BYTES, 0);
    WaveformEngine::mask_scanline::<P>(&mut out, 2, 6);
    assert_eq!(out[0], 0x0F);
    assert_eq!(out[1], 0xF0);
    assert!(out[2..].iter().all(|&b| b == 0));
    // Whole bytes only.
    let mut out = vec![0xFFu8; P::SCANLINE_PIXEL_BYTES];
    out.resize(P::SCANLINE_BYTES, 0);
    WaveformEngine::mask_scanline::<P>(&mut out, 8, 16);
    assert!(out[..2].iter().all(|&b| b == 0));
    assert_eq!(&out[2..4], &[0xFF, 0xFF]);
    assert!(out[4..].iter().all(|&b| b == 0));
    // Both edges in one byte.
    let mut out = vec![0xFFu8; P::SCANLINE_PIXEL_BYTES];
    WaveformEngine::mask_scanline::<P>(&mut out, 2, 4);
    assert_eq!(out[0], 0x0F);
    assert!(out[1..].iter().all(|&b| b == 0));
    // The full width changes nothing.
    let mut out = vec![0xFFu8; P::SCANLINE_PIXEL_BYTES];
    WaveformEngine::mask_scanline::<P>(&mut out, 0, 960);
    assert!(out.iter().all(|&b| b == 0xFF));
}
