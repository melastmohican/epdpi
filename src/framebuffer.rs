//! 4 bits per pixel framebuffer in caller-supplied memory.

use core::marker::PhantomData;

use crate::error::ValidationError;
use crate::traits::EpdPanel;

/// Framebuffer holding 16 gray levels (0 black, 15 white), two pixels per byte with the
/// left pixel in the high nibble.
///
/// With the `graphics` feature it implements `embedded-graphics` `DrawTarget` over `Gray4`,
/// so it can live in PSRAM while the display refreshes from another buffer.
pub struct EpdFrameBuffer<'a, PANEL: EpdPanel> {
    data: &'a mut [u8],
    _panel: PhantomData<PANEL>,
}

impl<'a, PANEL: EpdPanel> EpdFrameBuffer<'a, PANEL> {
    /// Wraps `slice`, which must hold at least [`EpdPanel::FB_BYTES`] bytes.
    pub fn new(slice: &'a mut [u8]) -> Result<Self, ValidationError> {
        if slice.len() < PANEL::FB_BYTES {
            return Err(ValidationError::BufferTooSmall {
                required: PANEL::FB_BYTES,
                provided: slice.len(),
            });
        }
        Ok(Self {
            data: slice,
            _panel: PhantomData,
        })
    }

    /// Raw framebuffer bytes, exactly [`EpdPanel::FB_BYTES`] long.
    pub fn as_bytes(&self) -> &[u8] {
        &self.data[..PANEL::FB_BYTES]
    }

    /// Mutable raw framebuffer bytes.
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.data[..PANEL::FB_BYTES]
    }

    /// Fills the whole buffer with one gray level (0 to 15).
    pub fn fill(&mut self, gray: u8) {
        let g = gray & 0x0f;
        self.as_bytes_mut().fill((g << 4) | g);
    }

    /// Reads one pixel, or `None` outside the panel.
    pub fn pixel(&self, x: u32, y: u32) -> Option<u8> {
        if x >= PANEL::WIDTH || y >= PANEL::HEIGHT {
            return None;
        }
        let byte = self.data[y as usize * PANEL::FB_STRIDE_BYTES + x as usize / 2];
        Some(if x % 2 == 0 { byte >> 4 } else { byte & 0x0f })
    }

    /// Writes one pixel. Coordinates outside the panel are ignored.
    pub fn set_pixel(&mut self, x: u32, y: u32, gray: u8) {
        if x >= PANEL::WIDTH || y >= PANEL::HEIGHT {
            return;
        }
        let idx = y as usize * PANEL::FB_STRIDE_BYTES + x as usize / 2;
        let g = gray & 0x0f;
        let byte = &mut self.data[idx];
        *byte = if x % 2 == 0 {
            (*byte & 0x0f) | (g << 4)
        } else {
            (*byte & 0xf0) | g
        };
    }
}

#[cfg(feature = "graphics")]
mod graphics {
    use core::convert::Infallible;

    use embedded_graphics_core::{
        Pixel,
        draw_target::DrawTarget,
        geometry::{OriginDimensions, Size},
        pixelcolor::{Gray4, GrayColor},
    };

    use super::EpdFrameBuffer;
    use crate::traits::EpdPanel;

    impl<PANEL: EpdPanel> OriginDimensions for EpdFrameBuffer<'_, PANEL> {
        fn size(&self) -> Size {
            Size::new(PANEL::WIDTH, PANEL::HEIGHT)
        }
    }

    impl<PANEL: EpdPanel> DrawTarget for EpdFrameBuffer<'_, PANEL> {
        type Color = Gray4;
        type Error = Infallible;

        fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
        where
            I: IntoIterator<Item = Pixel<Self::Color>>,
        {
            for Pixel(point, color) in pixels {
                if let (Ok(x), Ok(y)) = (u32::try_from(point.x), u32::try_from(point.y)) {
                    self.set_pixel(x, y, color.luma());
                }
            }
            Ok(())
        }
    }
}
