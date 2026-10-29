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
    /// 16-level update that follows M5GFX `epd_quality`: a short erase that moves each pixel
    /// from the gray it holds now towards mid gray, then the quality waveform for its target
    /// gray. No full clear, so the previous picture must be recorded (see
    /// [`EpdDisplay::clear`](crate::EpdDisplay::clear)).
    ///
    /// The waveform assumes a row data phase near 16 us. On a PaperDIY it gave an even ramp
    /// over an `LCD_CAM` DMA bus (22 us) and a squashed one over GPIO bit-banging (62 us).
    M5Quality,
    /// M5GFX `epd_text`: the erase, then `lut_text` (12 passes), but only for pixel pairs that
    /// are not white before and after. White stays untouched, so a page of text flashes less
    /// than [`M5Quality`](Self::M5Quality). Needs the record of the previous picture.
    M5Text,
    /// M5GFX `epd_fast`: the picture is reduced to black and white, pairs of pixels that changed
    /// get `lut_fast` (8 passes, no erase), the rest hold. The record then holds the black and
    /// white picture.
    M5Fast,
    /// M5GFX `epd_fastest`: like [`M5Fast`](Self::M5Fast) with `lut_fastest` (5 passes).
    M5Fastest,
    /// Flashes the panel black then white and leaves it white.
    Clear,
}

impl RefreshMode {
    /// True when the refresh reduces the picture to black and white, so the record of what the
    /// panel shows must be reduced the same way.
    pub(crate) fn is_binary(self) -> bool {
        matches!(self, Self::M5Fast | Self::M5Fastest)
    }
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
            RefreshMode::M5Quality => PANEL::ERASER_PASSES + PANEL::QUALITY_PASSES,
            RefreshMode::M5Text => PANEL::ERASER_PASSES + PANEL::TEXT_PASSES,
            RefreshMode::M5Fast => PANEL::FAST_PASSES,
            RefreshMode::M5Fastest => PANEL::FASTEST_PASSES,
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
        Self::lut_scanline::<PANEL>(fb_row, PANEL::WAVEFORM_LUT, PANEL::PASSES, pass, out);
    }

    /// Builds the scanline for `pass` of any `16 * passes` byte table, level-major, from one
    /// 4 bits per pixel row. The row picks each pixel's level, so the same call drives a
    /// table indexed by target gray (a framebuffer row) or by current gray (a record row).
    pub fn lut_scanline<PANEL: EpdPanel>(
        level_row: &[u8],
        lut: &[u8],
        passes: usize,
        pass: usize,
        out: &mut [u8],
    ) {
        let code = |gray: u8| {
            lut.get(usize::from(gray) * passes + pass)
                .copied()
                .unwrap_or(0)
        };
        for (dst, src) in out
            .iter_mut()
            .take(PANEL::SCANLINE_PIXEL_BYTES)
            .zip(level_row.chunks_exact(2))
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

    /// Clears the codes of every pixel outside `x0..x1` in a scanline, so those pixels hold. `x0`
    /// and `x1` are pixel columns, `x1` exclusive, both even.
    pub fn mask_scanline<PANEL: EpdPanel>(out: &mut [u8], x0: u32, x1: u32) {
        let end = PANEL::SCANLINE_PIXEL_BYTES.min(out.len());
        let first = (x0 as usize / 4).min(end);
        let last = (x1 as usize).div_ceil(4).clamp(first, end);
        out[..first].fill(0);
        out[last..end].fill(0);
        // Four pixels per byte, the first in bits 7:6. Keep the pixels from `x0` on in the first
        // byte and up to `x1` in the last one.
        let lead = x0 % 4;
        if lead > 0 && first < last {
            out[first] &= 0xff >> (2 * lead);
        }
        let trail = x1 % 4;
        if trail > 0 && last > first {
            out[last - 1] &= 0xffu8 << (8 - 2 * trail);
        }
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

    /// Builds the scanline for `pass` of one of the M5GFX update modes that only drive the pixel
    /// pairs that need it ([`RefreshMode::M5Text`], [`M5Fast`](RefreshMode::M5Fast) and
    /// [`M5Fastest`](RefreshMode::M5Fastest)). M5GFX tracks two pixels per byte, so a pair is
    /// driven as a whole: a pixel that did not change still gets its pair's waveform.
    ///
    /// Passes below `ERASER_PASSES` erase from the gray in `prev_row` (text mode only), the rest
    /// draw the gray in `next_row`. Any other mode fills the scanline with holds.
    pub fn m5_scanline<PANEL: EpdPanel>(
        mode: RefreshMode,
        prev_row: &[u8],
        next_row: &[u8],
        pass: usize,
        out: &mut [u8],
    ) {
        let (erase, lut, passes) = match mode {
            RefreshMode::M5Text => (true, PANEL::TEXT_LUT, PANEL::TEXT_PASSES),
            RefreshMode::M5Fast => (false, PANEL::FAST_LUT, PANEL::FAST_PASSES),
            RefreshMode::M5Fastest => (false, PANEL::FASTEST_LUT, PANEL::FASTEST_PASSES),
            _ => {
                Self::uniform_scanline::<PANEL>(DriveCode::Hold, out);
                return;
            }
        };
        // Most rows of a typical screen do not change. Text mode skips pairs that are white
        // before and after, the black and white modes pairs that are equal, so a row that is
        // all white (text) or identical (fast) holds everywhere and needs no per-pair work.
        let idle = if mode == RefreshMode::M5Text {
            prev_row.iter().all(|&b| b == 0xff) && next_row.iter().all(|&b| b == 0xff)
        } else {
            prev_row == next_row
        };
        if idle {
            Self::uniform_scanline::<PANEL>(DriveCode::Hold, out);
            return;
        }
        let binary = mode.is_binary();
        // Black and white modes see only the two end levels.
        let level = |nibble: u8| {
            if !binary {
                nibble
            } else if nibble >= MONO_THRESHOLD {
                0x0f
            } else {
                0
            }
        };
        let both = |byte: u8| (level(byte >> 4) << 4) | level(byte & 0x0f);
        let drawing = !erase || pass >= PANEL::ERASER_PASSES;
        let code = |nibble: u8| {
            if drawing {
                let step = if erase {
                    pass - PANEL::ERASER_PASSES
                } else {
                    pass
                };
                lut.get(usize::from(level(nibble)) * passes + step)
            } else {
                PANEL::ERASER_LUT.get(usize::from(nibble) * PANEL::ERASER_PASSES + pass)
            }
            .copied()
            .unwrap_or(0)
        };
        let active = |old: u8, new: u8| {
            if mode == RefreshMode::M5Text {
                !(old == 0xff && new == 0xff)
            } else {
                both(old) != both(new)
            }
        };
        let source = if drawing { next_row } else { prev_row };
        let pair = |index: usize| {
            let (old, new, src) = (prev_row[index], next_row[index], source[index]);
            if active(old, new) {
                [code(src >> 4), code(src & 0x0f)]
            } else {
                [0, 0]
            }
        };
        for (i, dst) in out.iter_mut().take(PANEL::SCANLINE_PIXEL_BYTES).enumerate() {
            let [a, b] = pair(2 * i);
            let [c, d] = pair(2 * i + 1);
            *dst = pack([a, b, c, d]);
        }
        clear_padding::<PANEL>(out);
    }
}
