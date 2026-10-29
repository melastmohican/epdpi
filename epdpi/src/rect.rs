//! Pixel rectangles for window updates.

use crate::traits::EpdPanel;

/// Axis aligned rectangle in panel pixels, origin top left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Rect {
    /// Left edge.
    pub x: u32,
    /// Top edge.
    pub y: u32,
    /// Width in pixels.
    pub w: u32,
    /// Height in pixels.
    pub h: u32,
}

impl Rect {
    /// A rectangle from its top left corner and size.
    pub const fn new(x: u32, y: u32, w: u32, h: u32) -> Self {
        Self { x, y, w, h }
    }

    /// The whole panel.
    pub const fn full<PANEL: EpdPanel>() -> Self {
        Self::new(0, 0, PANEL::WIDTH, PANEL::HEIGHT)
    }

    /// True when the rectangle covers no pixel.
    pub const fn is_empty(&self) -> bool {
        self.w == 0 || self.h == 0
    }

    /// The smallest rectangle holding both. An empty rectangle adds nothing.
    pub fn union(self, other: Self) -> Self {
        if self.is_empty() {
            return other;
        }
        if other.is_empty() {
            return self;
        }
        let x0 = self.x.min(other.x);
        let y0 = self.y.min(other.y);
        let x1 = self
            .x
            .saturating_add(self.w)
            .max(other.x.saturating_add(other.w));
        let y1 = self
            .y
            .saturating_add(self.h)
            .max(other.y.saturating_add(other.h));
        Self::new(x0, y0, x1 - x0, y1 - y0)
    }

    /// Clips to the panel and widens to whole pixel pairs, because the waveform engine drives
    /// two pixels at a time (as M5GFX does). `None` when nothing is left.
    pub fn aligned<PANEL: EpdPanel>(self) -> Option<Self> {
        let x0 = self.x.min(PANEL::WIDTH);
        let y0 = self.y.min(PANEL::HEIGHT);
        let x1 = self.x.saturating_add(self.w).min(PANEL::WIDTH);
        let y1 = self.y.saturating_add(self.h).min(PANEL::HEIGHT);
        if x1 <= x0 || y1 <= y0 {
            return None;
        }
        // WIDTH is a multiple of 4, so rounding the right edge up stays inside the panel.
        let x0 = x0 & !1;
        let x1 = (x1 + 1) & !1;
        Some(Self::new(x0, y0, x1 - x0, y1 - y0))
    }
}
