//! Turning 8 bit gray into the 4 bit levels of an [`EpdFrameBuffer`](crate::EpdFrameBuffer) the way M5GFX does.
//!
//! M5GFX keeps 4 bits per pixel but takes 8 bit gray from the caller, and dithers the 8 bits down with a
//! 4 by 4 Bayer matrix (`Panel_EPD.inl`). A flat 8 bit gray then comes out as a mix of two neighbouring
//! levels whose average is the gray you asked for, instead of the nearest level. `epdpi` takes levels as
//! they are, so a picture drawn with these helpers matches M5GFX's, and a picture drawn with plain
//! [`Gray4`](https://docs.rs/embedded-graphics-core/latest/embedded_graphics_core/pixelcolor/struct.Gray4.html)
//! values does not.
//!
//! Two reductions exist, one per kind of refresh:
//!
//! - [`gray8_to_level`] for the 16 level modes ([`RefreshMode::Quality16`](crate::RefreshMode::Quality16),
//!   [`M5Quality`](crate::RefreshMode::M5Quality), [`M5Text`](crate::RefreshMode::M5Text)).
//! - [`gray8_to_bw`] for the black and white modes ([`M5Fast`](crate::RefreshMode::M5Fast),
//!   [`M5Fastest`](crate::RefreshMode::M5Fastest)). It writes level 0 or 15, which those modes keep as
//!   they are, so the Bayer pattern shows a gray as a density of white pixels. `epdpi` itself cuts at
//!   level 8 when it reduces a picture, so it never does this for you.
//!
//! The pattern is placed by the pixel position, `(y & 3) * 4 + (x & 3)`. M5GFX uses panel pixels. The
//! [`EpdFrameBuffer`](crate::EpdFrameBuffer) methods use the position in the picture, which is the same
//! thing for [`Rotation::Deg0`](crate::Rotation::Deg0) and a different, equally even, pattern for the others.
//!
//! ```
//! use epdpi::{dither, Ed047tc1, EpdFrameBuffer, EpdPanel};
//!
//! let mut mem = vec![0u8; Ed047tc1::FB_BYTES];
//! let mut fb = EpdFrameBuffer::<Ed047tc1>::new(&mut mem).unwrap();
//! // A flat 8 bit gray of 119 (7 * 17): 15 of 16 pixels become level 7 and one becomes level 6.
//! for y in 0..4 {
//!     for x in 0..4 {
//!         fb.set_logical_pixel_gray8(x, y, 119);
//!     }
//! }
//! let sum: u32 = (0..4)
//!     .flat_map(|y| (0..4).map(move |x| (x, y)))
//!     .map(|(x, y)| u32::from(fb.logical_pixel(x, y).unwrap()))
//!     .sum();
//! assert_eq!(sum, 15 * 7 + 6);
//! assert_eq!(dither::gray8_to_level(119, 0, 0), 6);
//! ```

/// M5GFX's 4 by 4 Bayer matrix (`Bayer` in `Panel_EPD.inl`), read as `BAYER_4X4[(y & 3) * 4 + (x & 3)]`.
pub const BAYER_4X4: [u8; 16] = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];

const fn bayer(x: u32, y: u32) -> u8 {
    BAYER_4X4[(((y & 3) << 2) | (x & 3)) as usize]
}

/// Reduces 8 bit gray to one of the 16 levels (0 black to 15 white) at pixel `(x, y)`, the way M5GFX does
/// for its 16 level modes: `(gray + bayer - 8) >> 4`, kept between 0 and 15.
///
/// Averaged over the 16 positions of a tile, a flat gray `level * 17` gives `level` within 0.5: level 1
/// gives 0.56, level 7 gives 6.94, level 8 gives 8, level 9 gives 9.06 and level 14 gives 14.38.
pub const fn gray8_to_level(gray: u8, x: u32, y: u32) -> u8 {
    let v = gray as i16 + bayer(x, y) as i16 - 8;
    if v < 0 {
        0
    } else if v >= 256 {
        15
    } else {
        (v >> 4) as u8
    }
}

/// Reduces 8 bit gray to black (level 0) or white (level 15) at pixel `(x, y)`, the way M5GFX does for
/// [`M5Fast`](crate::RefreshMode::M5Fast) and [`M5Fastest`](crate::RefreshMode::M5Fastest): white when
/// `gray + 16 * bayer >= 248`.
///
/// For a flat gray `level * 17` the number of white pixels in a 4 by 4 tile is `level` for levels 0 to 7,
/// `level + 1` for levels 8 to 14 and 16 for level 15.
pub const fn gray8_to_bw(gray: u8, x: u32, y: u32) -> u8 {
    if gray as u16 + 16 * bayer(x, y) as u16 >= 248 {
        15
    } else {
        0
    }
}
