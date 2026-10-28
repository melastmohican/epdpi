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
}
