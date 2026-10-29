//! E Ink ED047TC1, 4.7 inch 960 x 540, 16 gray levels.
//!
//! Tested on the M5Stack PaperDIY.
//!
//! The waveform matrix is FastEPD's `u8M5Matrix` for this glass. M5GFX does not use it: it has
//! its own per-mode LUTs and an eraser pass. The matrix is indexed by target gray level only,
//! so it assumes every pixel starts from white. Run [`RefreshMode::Quality16`](crate::RefreshMode::Quality16), which clears
//! first, rather than driving the matrix from an unknown state.

use crate::traits::{ColorMode, EpdPanel};

/// 16 gray levels by 8 passes. Row 0 is black, row 15 is white.
/// 0 holds the pixel, 1 darkens it, 2 lightens it.
#[rustfmt::skip]
pub const ED047TC1_LUT: [u8; 128] = [
    1, 1, 1, 1, 1, 1, 1, 1,
    2, 2, 1, 1, 2, 1, 1, 1,
    2, 2, 1, 1, 1, 1, 2, 1,
    2, 2, 1, 1, 2, 2, 1, 1,
    2, 2, 2, 2, 1, 1, 2, 1,
    2, 2, 1, 1, 1, 2, 2, 1,
    2, 2, 1, 1, 2, 1, 1, 2,
    2, 2, 2, 1, 2, 1, 1, 2,
    2, 2, 2, 2, 2, 1, 2, 1,
    1, 1, 1, 1, 1, 1, 2, 2,
    2, 2, 1, 1, 1, 1, 2, 2,
    1, 1, 1, 1, 2, 1, 2, 2,
    2, 2, 1, 1, 2, 1, 2, 2,
    2, 1, 1, 2, 2, 1, 2, 2,
    2, 2, 1, 2, 2, 1, 2, 2,
    2, 2, 2, 2, 2, 2, 2, 2,
];

/// M5GFX `lut_eraser`, 16 gray levels by 2 passes. Row `g` is indexed by the gray level the
/// pixel holds now. Codes as for [`ED047TC1_LUT`] (M5GFX 3, no operation, is 0 here).
#[rustfmt::skip]
pub const M5_ERASER_LUT: [u8; 32] = [
    2, 2,
    2, 2,
    2, 3,
    2, 3,
    2, 3,
    2, 1,
    2, 1,
    2, 1,
    2, 1,
    2, 1,
    2, 1,
    3, 1,
    3, 1,
    3, 1,
    1, 1,
    1, 1,
];

/// M5GFX `lut_quality`, 16 gray levels by 15 passes. Row `g` is indexed by the target gray
/// level. M5GFX also pads the table with 16 no-operation passes, which are left out here.
#[rustfmt::skip]
pub const M5_QUALITY_LUT: [u8; 240] = [
    1, 2, 2, 1, 1, 1, 1, 1, 1, 3, 3, 1, 1, 3, 3,
    1, 2, 2, 1, 1, 1, 1, 1, 1, 2, 2, 1, 1, 1, 3,
    1, 2, 2, 1, 1, 1, 2, 1, 3, 2, 2, 1, 1, 1, 3,
    1, 2, 2, 1, 1, 1, 2, 1, 2, 2, 2, 1, 1, 1, 3,
    1, 2, 2, 1, 1, 1, 1, 1, 2, 2, 2, 3, 1, 1, 3,
    1, 2, 2, 1, 1, 1, 1, 2, 1, 2, 2, 2, 1, 1, 3,
    1, 2, 2, 1, 1, 1, 1, 1, 2, 2, 2, 2, 1, 1, 3,
    2, 2, 2, 1, 1, 1, 2, 1, 2, 2, 2, 2, 1, 1, 3,
    1, 2, 2, 1, 1, 2, 1, 2, 2, 2, 2, 2, 1, 1, 3,
    2, 2, 2, 1, 1, 1, 2, 2, 2, 2, 2, 2, 1, 1, 3,
    2, 2, 2, 1, 1, 1, 1, 1, 2, 2, 2, 2, 3, 1, 3,
    1, 2, 2, 1, 1, 1, 1, 2, 2, 2, 2, 2, 3, 1, 3,
    1, 2, 2, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 1, 2,
    1, 2, 2, 1, 1, 1, 1, 2, 1, 2, 2, 2, 2, 1, 2,
    1, 2, 2, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 1, 2,
    1, 2, 2, 1, 1, 1, 3, 2, 2, 2, 2, 2, 2, 3, 3,
];

/// M5GFX `lut_text`, 16 gray levels by 12 passes. Row `g` is indexed by the target gray level. M5GFX pads the table with 16 no-operation passes, left out here. Run after [`M5_ERASER_LUT`].
#[rustfmt::skip]
pub const M5_TEXT_LUT: [u8; 192] = [
    2, 2, 2, 2, 2, 1, 1, 1, 3, 1, 1, 1,
    2, 2, 2, 2, 2, 2, 3, 3, 1, 1, 1, 1,
    2, 2, 2, 2, 2, 2, 3, 3, 3, 1, 1, 1,
    2, 2, 2, 2, 2, 1, 1, 1, 2, 2, 1, 1,
    2, 2, 2, 2, 1, 1, 1, 2, 2, 2, 1, 1,
    2, 2, 2, 2, 2, 1, 1, 2, 2, 2, 1, 1,
    2, 2, 2, 2, 2, 1, 1, 1, 1, 2, 2, 1,
    2, 2, 2, 2, 2, 1, 3, 1, 1, 2, 2, 1,
    2, 2, 2, 2, 2, 3, 3, 1, 1, 2, 2, 1,
    2, 2, 2, 2, 2, 3, 1, 1, 2, 2, 2, 1,
    2, 2, 2, 2, 2, 1, 1, 2, 2, 2, 2, 1,
    2, 2, 2, 1, 1, 1, 1, 1, 1, 2, 2, 3,
    2, 2, 2, 2, 2, 3, 1, 1, 1, 2, 2, 3,
    2, 2, 2, 2, 2, 3, 3, 1, 1, 2, 2, 3,
    2, 2, 2, 2, 2, 1, 1, 1, 2, 2, 2, 3,
    1, 1, 1, 1, 1, 2, 2, 2, 3, 2, 2, 2,
];

/// M5GFX `lut_fast`, 16 gray levels by 8 passes. Only levels 0 and 15 are used, the picture is reduced to black and white first. No erase pass.
#[rustfmt::skip]
pub const M5_FAST_LUT: [u8; 128] = [
    2, 2, 1, 1, 1, 1, 1, 1,
    3, 3, 3, 3, 3, 3, 3, 3,
    3, 3, 3, 3, 3, 3, 3, 3,
    3, 3, 3, 3, 3, 3, 3, 3,
    3, 3, 3, 3, 3, 3, 3, 3,
    3, 3, 3, 3, 3, 3, 3, 3,
    3, 3, 3, 3, 3, 3, 3, 3,
    3, 3, 3, 3, 3, 3, 3, 3,
    3, 3, 3, 3, 3, 3, 3, 3,
    3, 3, 3, 3, 3, 3, 3, 3,
    3, 3, 3, 3, 3, 3, 3, 3,
    3, 3, 3, 3, 3, 3, 3, 3,
    3, 3, 3, 3, 3, 3, 3, 3,
    3, 3, 3, 3, 3, 3, 3, 3,
    3, 3, 3, 3, 3, 3, 3, 3,
    1, 1, 2, 2, 2, 2, 2, 2,
];

/// M5GFX `lut_fastest`, 16 gray levels by 5 passes. Black and white only, like [`M5_FAST_LUT`].
#[rustfmt::skip]
pub const M5_FASTEST_LUT: [u8; 80] = [
    2, 1, 1, 1, 1,
    3, 3, 3, 3, 3,
    3, 3, 3, 3, 3,
    3, 3, 3, 3, 3,
    3, 3, 3, 3, 3,
    3, 3, 3, 3, 3,
    3, 3, 3, 3, 3,
    3, 3, 3, 3, 3,
    3, 3, 3, 3, 3,
    3, 3, 3, 3, 3,
    3, 3, 3, 3, 3,
    3, 3, 3, 3, 3,
    3, 3, 3, 3, 3,
    3, 3, 3, 3, 3,
    3, 3, 3, 3, 3,
    1, 2, 2, 2, 2,
];

/// E Ink ED047TC1 panel.
#[derive(Debug, Clone, Copy, Default)]
pub struct Ed047tc1;

impl EpdPanel for Ed047tc1 {
    const WIDTH: u32 = 960;
    const HEIGHT: u32 = 540;
    const COLOR_MODE: ColorMode = ColorMode::Grayscale16;
    const VCOM_MV: Option<i16> = Some(-2000);
    const LINE_PADDING: usize = 8;
    const WAVEFORM_LUT: &'static [u8] = &ED047TC1_LUT;
    const ERASER_LUT: &'static [u8] = &M5_ERASER_LUT;
    const QUALITY_LUT: &'static [u8] = &M5_QUALITY_LUT;
    const TEXT_LUT: &'static [u8] = &M5_TEXT_LUT;
    const FAST_LUT: &'static [u8] = &M5_FAST_LUT;
    const FASTEST_LUT: &'static [u8] = &M5_FASTEST_LUT;
}
