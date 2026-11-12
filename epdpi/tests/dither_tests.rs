#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! The M5GFX dither helpers: the matrix, the means of the 16 level reduction (M5GFX audit F7), the white
//! densities of the black and white reduction (F6) and the framebuffer methods.

use epdpi::dither::{BAYER_4X4, gray8_to_bw, gray8_to_level};
use epdpi::prelude::*;

type P = Ed047tc1;

/// Sum of `f` over the 16 positions of a 4 by 4 tile.
fn tile_sum(f: impl Fn(u32, u32) -> u8) -> u32 {
    (0..4)
        .flat_map(|y| (0..4).map(move |x| (x, y)))
        .map(|(x, y)| u32::from(f(x, y)))
        .sum()
}

#[test]
fn matrix_is_the_m5gfx_one() {
    assert_eq!(
        BAYER_4X4,
        [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5]
    );
    let mut sorted = BAYER_4X4;
    sorted.sort_unstable();
    assert_eq!(sorted, core::array::from_fn(|i| i as u8));
}

#[test]
fn flat_ramp_means_match_the_audit() {
    // `EpdQualityRamp` input: gray = level * 17. Mean level over a tile, times 16, as an integer.
    let sum = |level: u8| tile_sum(|x, y| gray8_to_level(level * 17, x, y));
    assert_eq!(sum(1), 9); // 0.5625, -0.44
    assert_eq!(sum(7), 111); // 6.9375, -0.06
    assert_eq!(sum(8), 128); // 8.0
    assert_eq!(sum(9), 145); // 9.0625, +0.06
    assert_eq!(sum(14), 230); // 14.375, +0.38 (level 14 gives 14.38)
    assert_eq!(sum(0), 0);
    assert_eq!(sum(15), 15 * 16);
}

#[test]
fn level_reduction_is_monotonic_and_covers_the_ends() {
    for y in 0..4 {
        for x in 0..4 {
            assert_eq!(gray8_to_level(0, x, y), 0);
            assert_eq!(gray8_to_level(255, x, y), 15);
            let mut prev = 0;
            for g in 0..=255u8 {
                let l = gray8_to_level(g, x, y);
                assert!(l >= prev && l <= 15);
                prev = l;
            }
        }
    }
}

#[test]
fn black_and_white_densities_match_the_audit() {
    for level in 0u8..=15 {
        let white = tile_sum(|x, y| gray8_to_bw(level * 17, x, y)) / 15;
        let expect = match level {
            0..=7 => u32::from(level),
            8..=14 => u32::from(level) + 1,
            _ => 16,
        };
        assert_eq!(white, expect, "level {level}");
    }
}

#[test]
fn black_and_white_only_gives_0_or_15() {
    for g in 0..=255u8 {
        for y in 0..4 {
            for x in 0..4 {
                assert!(matches!(gray8_to_bw(g, x, y), 0 | 15));
            }
        }
    }
}

#[test]
fn pattern_repeats_every_four_pixels() {
    for g in [3u8, 100, 119, 200, 247] {
        for y in 0..4 {
            for x in 0..4 {
                assert_eq!(gray8_to_level(g, x, y), gray8_to_level(g, x + 8, y + 12));
                assert_eq!(gray8_to_bw(g, x, y), gray8_to_bw(g, x + 4, y + 4));
            }
        }
    }
}

#[test]
fn framebuffer_methods_write_the_helper_values() {
    let mut buf = vec![0u8; P::FB_BYTES];
    let mut fb = EpdFrameBuffer::<P>::new(&mut buf).unwrap();
    for (x, y) in [(0, 0), (1, 0), (5, 3), (959, 539)] {
        fb.set_logical_pixel_gray8(x, y, 119);
        assert_eq!(fb.logical_pixel(x, y), Some(gray8_to_level(119, x, y)));
        fb.set_logical_pixel_bw8(x, y, 119);
        assert_eq!(fb.logical_pixel(x, y), Some(gray8_to_bw(119, x, y)));
    }
    // Outside the picture is ignored.
    fb.set_logical_pixel_gray8(960, 0, 200);
    fb.set_logical_pixel_bw8(0, 540, 200);
}

#[test]
fn rotated_picture_places_the_pattern_by_picture_position() {
    let mut buf = vec![0u8; P::FB_BYTES];
    let mut fb = EpdFrameBuffer::<P>::new(&mut buf).unwrap();
    fb.set_rotation(Rotation::Deg90);
    let (w, h) = fb.logical_size();
    for (x, y) in [(0, 0), (3, 1), (w - 1, h - 1)] {
        fb.set_logical_pixel_gray8(x, y, 119);
        assert_eq!(fb.logical_pixel(x, y), Some(gray8_to_level(119, x, y)));
    }
}
