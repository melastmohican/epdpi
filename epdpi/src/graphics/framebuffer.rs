//! 4 bits per pixel framebuffer in caller-supplied memory.

use core::marker::PhantomData;

use super::rect::Rect;
use super::rotation::Rotation;
use crate::error::ValidationError;
use crate::traits::EpdPanel;

/// Framebuffer holding 16 gray levels (0 black, 15 white), two pixels per byte with the
/// left pixel in the high nibble.
///
/// With the `graphics` feature it implements `embedded-graphics` `DrawTarget` over `Gray4`,
/// so it can live in PSRAM while the display refreshes from another buffer. The `DrawTarget`
/// follows [`set_rotation`](Self::set_rotation); `set_pixel`, `pixel`, the dirty area and the
/// bytes always use panel coordinates.
///
/// It also keeps the bounding box of everything written since the last
/// [`clear_dirty`](Self::clear_dirty), for
/// [`EpdDisplay::refresh_dirty`](crate::EpdDisplay::refresh_dirty). Writing through
/// [`as_bytes_mut`](Self::as_bytes_mut) marks the whole buffer dirty, since the writes cannot be
/// tracked.
pub struct EpdFrameBuffer<'a, PANEL: EpdPanel> {
    data: &'a mut [u8],
    dirty: Rect,
    rotation: Rotation,
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
            // A new buffer has not been shown yet.
            dirty: Rect::full::<PANEL>(),
            rotation: Rotation::Deg0,
            _panel: PhantomData,
        })
    }

    /// Turns the picture that the `DrawTarget` and the `*_logical` methods draw. Does not move
    /// pixels already in the buffer, so set it before drawing.
    pub fn set_rotation(&mut self, rotation: Rotation) {
        self.rotation = rotation;
    }

    /// The current orientation, [`Rotation::Deg0`] for a new buffer.
    pub fn rotation(&self) -> Rotation {
        self.rotation
    }

    /// Width and height of the picture in the current orientation.
    pub fn logical_size(&self) -> (u32, u32) {
        self.rotation.size::<PANEL>()
    }

    /// Converts a rectangle of the picture to panel pixels, for
    /// [`mark_dirty`](Self::mark_dirty) and
    /// [`EpdDisplay::refresh_rect`](crate::EpdDisplay::refresh_rect), which take panel pixels.
    pub fn rect_to_panel(&self, rect: impl Into<Rect>) -> Rect {
        self.rotation.rect_to_panel::<PANEL>(rect.into())
    }

    /// Reads one pixel of the picture in the current orientation.
    pub fn logical_pixel(&self, x: u32, y: u32) -> Option<u8> {
        let (px, py) = self.rotation.to_panel::<PANEL>(x, y)?;
        self.pixel(px, py)
    }

    /// Writes one pixel of the picture in the current orientation. Outside it is ignored.
    pub fn set_logical_pixel(&mut self, x: u32, y: u32, gray: u8) {
        if let Some((px, py)) = self.rotation.to_panel::<PANEL>(x, y) {
            self.set_pixel(px, py, gray);
        }
    }

    /// Raw framebuffer bytes, exactly [`EpdPanel::FB_BYTES`] long.
    pub fn as_bytes(&self) -> &[u8] {
        &self.data[..PANEL::FB_BYTES]
    }

    /// Mutable raw framebuffer bytes. Marks the whole buffer dirty.
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        self.dirty = Rect::full::<PANEL>();
        &mut self.data[..PANEL::FB_BYTES]
    }

    /// The bounding box of everything written since the last [`clear_dirty`](Self::clear_dirty),
    /// or `None` when nothing was. A new buffer starts out fully dirty.
    pub fn dirty(&self) -> Option<Rect> {
        if self.dirty.is_empty() {
            None
        } else {
            Some(self.dirty)
        }
    }

    /// Adds `rect` to the dirty area, for callers that changed pixels another way.
    pub fn mark_dirty(&mut self, rect: impl Into<Rect>) {
        if let Some(rect) = rect.into().aligned::<PANEL>() {
            self.dirty = self.dirty.union(rect);
        }
    }

    /// Forgets the dirty area, as after showing it.
    pub fn clear_dirty(&mut self) {
        self.dirty = Rect::new(0, 0, 0, 0);
    }

    /// Fills the whole buffer with one gray level (0 to 15).
    pub fn fill(&mut self, gray: u8) {
        let g = gray & 0x0f;
        self.as_bytes_mut().fill((g << 4) | g);
    }

    /// Reads one pixel in panel coordinates, or `None` outside the panel.
    pub fn pixel(&self, x: u32, y: u32) -> Option<u8> {
        if x >= PANEL::WIDTH || y >= PANEL::HEIGHT {
            return None;
        }
        let byte = self.data[y as usize * PANEL::FB_STRIDE_BYTES + x as usize / 2];
        Some(if x % 2 == 0 { byte >> 4 } else { byte & 0x0f })
    }

    /// Writes one pixel in panel coordinates. Coordinates outside the panel are ignored.
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
        self.dirty = self.dirty.union(Rect::new(x, y, 1, 1));
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
            let (w, h) = self.logical_size();
            Size::new(w, h)
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
                    self.set_logical_pixel(x, y, color.luma());
                }
            }
            Ok(())
        }
    }
}
