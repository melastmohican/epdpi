//! Panel and bus traits.

/// Color capability of a panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ColorMode {
    /// Black and white only.
    BlackWhite,
    /// Sixteen gray levels, 4 bits per pixel in the framebuffer.
    #[default]
    Grayscale16,
}

/// Drive code sent to the source drivers for one pixel during one pass.
///
/// Two bits per pixel go over the bus, four pixels per byte, first pixel in bits 7:6.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
pub enum DriveCode {
    /// No voltage across the pixel, it keeps its current shade.
    Hold = 0,
    /// Push the pixel towards black.
    Darken = 1,
    /// Push the pixel towards white.
    Lighten = 2,
}

/// Static physical facts about one raw-parallel panel, held by a zero-sized type.
pub trait EpdPanel {
    /// Physical width of the panel in pixels. Must be a multiple of 4.
    const WIDTH: u32;

    /// Physical height of the panel in pixels.
    const HEIGHT: u32;

    /// Color capability of the panel.
    const COLOR_MODE: ColorMode = ColorMode::Grayscale16;

    /// Factory target VCOM in millivolts, for boards that program it through a PMIC.
    const VCOM_MV: Option<i16> = None;

    /// Dummy bytes clocked after each row's pixel data.
    const LINE_PADDING: usize = 0;

    /// Bytes per framebuffer row at 4 bits per pixel.
    const FB_STRIDE_BYTES: usize = Self::WIDTH as usize / 2;

    /// Bytes in one 4 bits per pixel framebuffer.
    const FB_BYTES: usize = Self::FB_STRIDE_BYTES * Self::HEIGHT as usize;

    /// Bytes of pixel data per bus scanline, at 2 bits per pixel.
    const SCANLINE_PIXEL_BYTES: usize = Self::WIDTH as usize / 4;

    /// Bytes per bus scanline including trailing dummy clocks.
    const SCANLINE_BYTES: usize = Self::SCANLINE_PIXEL_BYTES + Self::LINE_PADDING;

    /// Waveform matrix, `16 * PASSES` bytes. Row `g` (gray level `g`, 0 black to 15 white)
    /// holds one [`DriveCode`] per pass.
    const WAVEFORM_LUT: &'static [u8];

    /// Number of passes in [`WAVEFORM_LUT`](Self::WAVEFORM_LUT).
    const PASSES: usize = Self::WAVEFORM_LUT.len() / 16;

    /// Erase passes of the M5GFX style update, `16 * ERASER_PASSES` bytes. Row `g` is indexed
    /// by the gray level a pixel holds now, so any old picture is first moved towards mid
    /// gray. Empty when the panel has no such table.
    const ERASER_LUT: &'static [u8] = &[];

    /// Number of passes in [`ERASER_LUT`](Self::ERASER_LUT).
    const ERASER_PASSES: usize = Self::ERASER_LUT.len() / 16;

    /// Drawing passes of the M5GFX style update, `16 * QUALITY_PASSES` bytes. Row `g` is
    /// indexed by the target gray level. Empty when the panel has no such table.
    const QUALITY_LUT: &'static [u8] = &[];

    /// Number of passes in [`QUALITY_LUT`](Self::QUALITY_LUT).
    const QUALITY_PASSES: usize = Self::QUALITY_LUT.len() / 16;
}

/// Hardware transport for a raw-parallel panel: an 8-bit data bus plus the gate and source
/// control lines.
///
/// A frame pass is `start_frame`, then for every row `write_scanline` followed by `next_row`,
/// then `end_frame`.
#[allow(async_fn_in_trait)]
#[maybe_async_cfg::maybe(
    sync(feature = "blocking", keep_self),
    async(not(feature = "blocking"), keep_self)
)]
pub trait EpdParallelBus {
    /// Error emitted by the underlying pins or DMA peripheral.
    type Error;

    /// Switches the panel bias rails on or off, with the settle delays the panel needs.
    async fn set_power(&mut self, on: bool) -> Result<(), Self::Error>;

    /// Pulses SPV and CKV to start a vertical scan at the first row.
    async fn start_frame(&mut self) -> Result<(), Self::Error>;

    /// Streams one scanline of 2 bits per pixel drive codes, padding included, into the
    /// source drivers.
    async fn write_scanline(&mut self, data: &[u8]) -> Result<(), Self::Error>;

    /// Latches the scanline just written and clocks the gate drivers to the next row.
    async fn next_row(&mut self) -> Result<(), Self::Error>;

    /// Ends the vertical scan and lets the panel drivers settle.
    async fn end_frame(&mut self) -> Result<(), Self::Error>;
}
