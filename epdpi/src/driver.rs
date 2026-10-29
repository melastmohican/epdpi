//! Display orchestrator and builder.

use core::marker::PhantomData;

use crate::error::ValidationError;
use crate::framebuffer::EpdFrameBuffer;
use crate::rect::Rect;
use crate::traits::{DriveCode, EpdPanel, EpdParallelBus};
use crate::waveform::{CLEAR_PASSES, RefreshMode, WaveformEngine};

/// Builds an [`EpdDisplay`].
#[derive(Debug)]
pub struct EpdDisplayBuilder<PANEL> {
    vcom_mv: Option<i16>,
    _panel: PhantomData<PANEL>,
}

impl<PANEL: EpdPanel> Default for EpdDisplayBuilder<PANEL> {
    fn default() -> Self {
        Self::new()
    }
}

impl<PANEL: EpdPanel> EpdDisplayBuilder<PANEL> {
    /// Starts a builder with the panel's factory settings.
    pub fn new() -> Self {
        Self {
            vcom_mv: None,
            _panel: PhantomData,
        }
    }

    /// Overrides the VCOM target in millivolts. The driver only records it, boards that
    /// program VCOM through a PMIC read it back with [`EpdDisplay::vcom_mv`].
    pub fn with_vcom(mut self, vcom_mv: i16) -> Self {
        self.vcom_mv = Some(vcom_mv);
        self
    }

    /// Finishes the display.
    ///
    /// `prev_buffer` is the 4 bits per pixel record of what the glass currently shows, at
    /// least [`EpdPanel::FB_BYTES`] long. `scanline` is the bus line buffer, at least
    /// [`EpdPanel::SCANLINE_BYTES`] long. If a DMA bus needs it, it must live in DMA memory.
    ///
    /// The panel's contents are unknown at power-up, so run [`EpdDisplay::clear`] before
    /// the first refresh.
    pub fn build<'a, BUS: EpdParallelBus>(
        self,
        bus: BUS,
        prev_buffer: &'a mut [u8],
        scanline: &'a mut [u8],
    ) -> Result<EpdDisplay<'a, BUS, PANEL>, ValidationError> {
        if prev_buffer.len() < PANEL::FB_BYTES {
            return Err(ValidationError::BufferTooSmall {
                required: PANEL::FB_BYTES,
                provided: prev_buffer.len(),
            });
        }
        if scanline.len() < PANEL::SCANLINE_BYTES {
            return Err(ValidationError::BufferTooSmall {
                required: PANEL::SCANLINE_BYTES,
                provided: scanline.len(),
            });
        }
        Ok(EpdDisplay {
            bus,
            prev_buffer,
            scanline,
            vcom_mv: self.vcom_mv.or(PANEL::VCOM_MV).unwrap_or(-2000),
            _panel: PhantomData,
        })
    }
}

/// Raw-parallel e-paper display driver.
pub struct EpdDisplay<'a, BUS, PANEL> {
    bus: BUS,
    prev_buffer: &'a mut [u8],
    scanline: &'a mut [u8],
    vcom_mv: i16,
    _panel: PhantomData<PANEL>,
}

impl<BUS, PANEL> EpdDisplay<'_, BUS, PANEL> {
    /// VCOM target in millivolts the display was built with.
    pub fn vcom_mv(&self) -> i16 {
        self.vcom_mv
    }

    /// Gives the bus back.
    pub fn release(self) -> BUS {
        self.bus
    }
}

#[maybe_async_cfg::maybe(
    sync(feature = "blocking", keep_self),
    async(not(feature = "blocking"), keep_self)
)]
impl<BUS: EpdParallelBus, PANEL: EpdPanel> EpdDisplay<'_, BUS, PANEL> {
    /// Drives every pixel with `code` for `passes` passes.
    async fn drive_uniform(&mut self, code: DriveCode, passes: usize) -> Result<(), BUS::Error> {
        WaveformEngine::uniform_scanline::<PANEL>(code, self.scanline);
        let line = &self.scanline[..PANEL::SCANLINE_BYTES];
        for _ in 0..passes {
            self.bus.start_frame().await?;
            for _ in 0..PANEL::HEIGHT {
                self.bus.write_scanline(line).await?;
                self.bus.next_row().await?;
            }
            self.bus.end_frame().await?;
        }
        Ok(())
    }

    /// Flashes black then white, leaving the panel white.
    async fn clear_cycle(&mut self) -> Result<(), BUS::Error> {
        self.drive_uniform(DriveCode::Darken, CLEAR_PASSES).await?;
        self.drive_uniform(DriveCode::Lighten, CLEAR_PASSES).await?;
        // Neutral pass puts the drivers into their discharge state.
        self.drive_uniform(DriveCode::Hold, 1).await
    }

    /// Flashes the panel to a clean white and resets the record of what it shows.
    pub async fn clear(&mut self) -> Result<(), BUS::Error> {
        self.bus.set_power(true).await?;
        let result = self.clear_cycle().await;
        let off = self.bus.set_power(false).await;
        result?;
        off?;
        self.prev_buffer[..PANEL::FB_BYTES].fill(0xff);
        Ok(())
    }

    /// Shows `fb` using `mode`, then records it as the panel's current contents.
    pub async fn refresh(
        &mut self,
        fb: &EpdFrameBuffer<'_, PANEL>,
        mode: RefreshMode,
    ) -> Result<(), BUS::Error> {
        self.refresh_window(fb, mode, Rect::full::<PANEL>()).await
    }

    /// Like [`refresh`](Self::refresh) but only for the pixels inside `rect`, the way M5GFX's
    /// `display(x, y, w, h)` works. Everything outside holds, and the record of what the glass
    /// shows is only updated inside, so changes made to `fb` elsewhere are neither shown nor
    /// forgotten: a later refresh of that area still sees them.
    ///
    /// `rect` is clipped to the panel and widened to whole pixel pairs. An empty rectangle
    /// does nothing, not even a power cycle. Every row is still clocked, so a window is not
    /// faster than a full refresh with the same mode; the gain is that nothing outside it
    /// is driven and the passes run only for it.
    ///
    /// [`Quality16`](RefreshMode::Quality16) and [`Clear`](RefreshMode::Clear) always cover the
    /// whole panel, `rect` is ignored for them.
    pub async fn refresh_rect(
        &mut self,
        fb: &EpdFrameBuffer<'_, PANEL>,
        mode: RefreshMode,
        rect: Rect,
    ) -> Result<(), BUS::Error> {
        if matches!(mode, RefreshMode::Quality16 | RefreshMode::Clear) {
            return self.refresh(fb, mode).await;
        }
        let Some(window) = rect.aligned::<PANEL>() else {
            return Ok(());
        };
        self.refresh_window(fb, mode, window).await
    }

    /// Refreshes only the area of `fb` written since the last call, like M5GFX's `display()`
    /// with no arguments, then forgets it. Does nothing when nothing changed. A buffer that
    /// was just created counts as fully dirty. On an error the dirty area is kept, so the call
    /// can be repeated.
    ///
    /// Only drawing and [`EpdFrameBuffer::mark_dirty`] are tracked. A plain
    /// [`refresh`](Self::refresh) does not clear the dirty area.
    pub async fn refresh_dirty(
        &mut self,
        fb: &mut EpdFrameBuffer<'_, PANEL>,
        mode: RefreshMode,
    ) -> Result<(), BUS::Error> {
        let Some(area) = fb.dirty() else {
            return Ok(());
        };
        self.refresh_rect(fb, mode, area).await?;
        fb.clear_dirty();
        Ok(())
    }

    async fn refresh_window(
        &mut self,
        fb: &EpdFrameBuffer<'_, PANEL>,
        mode: RefreshMode,
        window: Rect,
    ) -> Result<(), BUS::Error> {
        self.bus.set_power(true).await?;
        let result = self.run_refresh(fb, mode, window).await;
        let off = self.bus.set_power(false).await;
        result?;
        off?;
        let record = &mut self.prev_buffer[..PANEL::FB_BYTES];
        if mode == RefreshMode::Clear {
            record.fill(0xff);
            return Ok(());
        }
        let stride = PANEL::FB_STRIDE_BYTES;
        let bytes = window.x as usize / 2..(window.x + window.w) as usize / 2;
        for row in window.y as usize..(window.y + window.h) as usize {
            let span = row * stride + bytes.start..row * stride + bytes.end;
            let src = &fb.as_bytes()[span.clone()];
            if mode.is_binary() {
                for (dst, src) in record[span].iter_mut().zip(src) {
                    *dst = binary_pair(*src);
                }
            } else {
                record[span].copy_from_slice(src);
            }
        }
        Ok(())
    }

    async fn run_refresh(
        &mut self,
        fb: &EpdFrameBuffer<'_, PANEL>,
        mode: RefreshMode,
        window: Rect,
    ) -> Result<(), BUS::Error> {
        if mode == RefreshMode::Clear {
            return self.clear_cycle().await;
        }
        if mode == RefreshMode::Quality16 {
            self.drive_uniform(DriveCode::Darken, CLEAR_PASSES).await?;
            self.drive_uniform(DriveCode::Lighten, CLEAR_PASSES).await?;
        }
        let stride = PANEL::FB_STRIDE_BYTES;
        let rows = window.y as usize..(window.y + window.h) as usize;
        let columns = window.x..window.x + window.w;
        let whole_width = window.x == 0 && window.w == PANEL::WIDTH;
        for pass in 0..WaveformEngine::passes::<PANEL>(mode) {
            self.bus.start_frame().await?;
            for row in 0..PANEL::HEIGHT as usize {
                if !rows.contains(&row) {
                    // Outside the window every pixel holds.
                    WaveformEngine::uniform_scanline::<PANEL>(DriveCode::Hold, self.scanline);
                    self.bus
                        .write_scanline(&self.scanline[..PANEL::SCANLINE_BYTES])
                        .await?;
                    self.bus.next_row().await?;
                    continue;
                }
                let range = row * stride..(row + 1) * stride;
                let next = &fb.as_bytes()[range.clone()];
                match mode {
                    RefreshMode::FastMono => WaveformEngine::mono_scanline::<PANEL>(
                        &self.prev_buffer[range],
                        next,
                        self.scanline,
                    ),
                    RefreshMode::M5Quality if pass < PANEL::ERASER_PASSES => {
                        WaveformEngine::lut_scanline::<PANEL>(
                            &self.prev_buffer[range],
                            PANEL::ERASER_LUT,
                            PANEL::ERASER_PASSES,
                            pass,
                            self.scanline,
                        );
                    }
                    RefreshMode::M5Quality => WaveformEngine::lut_scanline::<PANEL>(
                        next,
                        PANEL::QUALITY_LUT,
                        PANEL::QUALITY_PASSES,
                        pass - PANEL::ERASER_PASSES,
                        self.scanline,
                    ),
                    RefreshMode::M5Text | RefreshMode::M5Fast | RefreshMode::M5Fastest => {
                        WaveformEngine::m5_scanline::<PANEL>(
                            mode,
                            &self.prev_buffer[range],
                            next,
                            pass,
                            self.scanline,
                        );
                    }
                    _ => WaveformEngine::gray_scanline::<PANEL>(next, pass, self.scanline),
                }
                if !whole_width {
                    WaveformEngine::mask_scanline::<PANEL>(
                        self.scanline,
                        columns.start,
                        columns.end,
                    );
                }
                self.bus
                    .write_scanline(&self.scanline[..PANEL::SCANLINE_BYTES])
                    .await?;
                self.bus.next_row().await?;
            }
            self.bus.end_frame().await?;
        }
        self.drive_uniform(DriveCode::Hold, 1).await
    }
}

/// Reduces two packed 4 bit pixels to black or white, the way the black and white M5GFX modes
/// see them.
fn binary_pair(byte: u8) -> u8 {
    let level = |nibble: u8| if nibble >= 8 { 0x0f } else { 0 };
    (level(byte >> 4) << 4) | level(byte & 0x0f)
}
