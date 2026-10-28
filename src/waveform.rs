//! Stateless waveform engine: turns framebuffer rows into per-pass scanlines.

use crate::traits::{DriveCode, EpdPanel};

/// How a refresh drives the panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum RefreshMode {
    /// Differential black and white update. Only pixels that changed since the last refresh
    /// are driven, so there is no full-screen flash. Pixels at gray level 8 or above count
    /// as white. Ghosting builds up, so follow a run of these with [`Clear`](Self::Clear).
    FastMono,
    /// Full 16-level update: clears the panel, then runs every pass of the panel's waveform
    /// matrix. Flashes and takes several seconds.
    Quality16,
    /// Flashes the panel black then white and leaves it white.
    Clear,
}

/// Darken and lighten pulses in the clear cycle.
pub(crate) const CLEAR_PASSES: usize = 8;
/// Passes in a [`RefreshMode::FastMono`] update.
pub(crate) const MONO_PASSES: usize = 4;
/// Gray level at and above which [`RefreshMode::FastMono`] treats a pixel as white.
const MONO_THRESHOLD: u8 = 8;

/// Universal waveform engine. Every panel-specific number lives on [`EpdPanel`].
#[derive(Debug, Clone, Copy)]
pub struct WaveformEngine;

fn pack(codes: [u8; 4]) -> u8 {
    (codes[0] << 6) | (codes[1] << 4) | (codes[2] << 2) | codes[3]
}

fn clear_padding<PANEL: EpdPanel>(out: &mut [u8]) {
    let end = PANEL::SCANLINE_BYTES.min(out.len());
    let start = PANEL::SCANLINE_PIXEL_BYTES.min(end);
    out[start..end].fill(0);
}

impl WaveformEngine {
    /// Number of waveform passes `mode` runs after any clear cycle.
    pub fn passes<PANEL: EpdPanel>(mode: RefreshMode) -> usize {
        match mode {
            RefreshMode::FastMono => MONO_PASSES,
            RefreshMode::Quality16 => PANEL::PASSES,
            RefreshMode::Clear => 0,
        }
    }

    /// Fills `out` with one drive code for every pixel, for clear and discharge passes.
    pub fn uniform_scanline<PANEL: EpdPanel>(code: DriveCode, out: &mut [u8]) {
        let c = code as u8;
        let end = PANEL::SCANLINE_PIXEL_BYTES.min(out.len());
        out[..end].fill(pack([c; 4]));
        clear_padding::<PANEL>(out);
    }

    /// Builds the scanline for `pass` of the panel's waveform matrix from one 4 bits per
    /// pixel framebuffer row (left pixel in the high nibble).
    pub fn gray_scanline<PANEL: EpdPanel>(fb_row: &[u8], pass: usize, out: &mut [u8]) {
        let passes = PANEL::PASSES;
        let lut = PANEL::WAVEFORM_LUT;
        let code = |gray: u8| {
            lut.get(usize::from(gray) * passes + pass)
                .copied()
                .unwrap_or(0)
        };
        for (dst, src) in out
            .iter_mut()
            .take(PANEL::SCANLINE_PIXEL_BYTES)
            .zip(fb_row.chunks_exact(2))
        {
            *dst = pack([
                code(src[0] >> 4),
                code(src[0] & 0x0f),
                code(src[1] >> 4),
                code(src[1] & 0x0f),
            ]);
        }
        clear_padding::<PANEL>(out);
    }

    /// Builds a differential black and white scanline: unchanged pixels hold, changed
    /// pixels are driven to black or white.
    pub fn mono_scanline<PANEL: EpdPanel>(prev_row: &[u8], next_row: &[u8], out: &mut [u8]) {
        let is_white = |gray: u8| gray >= MONO_THRESHOLD;
        let code = |old: u8, new: u8| {
            if is_white(old) == is_white(new) {
                DriveCode::Hold as u8
            } else if is_white(new) {
                DriveCode::Lighten as u8
            } else {
                DriveCode::Darken as u8
            }
        };
        for (dst, (p, n)) in out
            .iter_mut()
            .take(PANEL::SCANLINE_PIXEL_BYTES)
            .zip(prev_row.chunks_exact(2).zip(next_row.chunks_exact(2)))
        {
            *dst = pack([
                code(p[0] >> 4, n[0] >> 4),
                code(p[0] & 0x0f, n[0] & 0x0f),
                code(p[1] >> 4, n[1] >> 4),
                code(p[1] & 0x0f, n[1] & 0x0f),
            ]);
        }
        clear_padding::<PANEL>(out);
    }
}
