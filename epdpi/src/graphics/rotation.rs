//! Screen orientation for the drawing surface.

use super::rect::Rect;
use crate::traits::EpdPanel;

/// How the picture is turned onto the panel, clockwise.
///
/// The ED047TC1 is wired landscape, 960 by 540. [`Deg90`](Self::Deg90) and
/// [`Deg270`](Self::Deg270) give a 540 by 960 portrait picture, like M5GFX's default on the
/// PaperDIY. Only the `DrawTarget` and the `*_logical` helpers of
/// [`EpdFrameBuffer`](super::EpdFrameBuffer) turn the picture. The raw pixel access and the
/// buffer bytes always use panel coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Rotation {
    /// Panel orientation, 960 by 540. The default.
    #[default]
    Deg0,
    /// Turned 90 degrees clockwise: the top of the picture is the right edge of the panel.
    Deg90,
    /// Upside down.
    Deg180,
    /// Turned 270 degrees clockwise: the top of the picture is the left edge of the panel.
    Deg270,
}

impl Rotation {
    /// True for the two portrait orientations, where width and height swap.
    pub const fn swaps_axes(self) -> bool {
        matches!(self, Self::Deg90 | Self::Deg270)
    }

    /// Width and height of the picture.
    pub const fn size<PANEL: EpdPanel>(self) -> (u32, u32) {
        if self.swaps_axes() {
            (PANEL::HEIGHT, PANEL::WIDTH)
        } else {
            (PANEL::WIDTH, PANEL::HEIGHT)
        }
    }

    /// Maps a picture pixel to a panel pixel. `None` outside the picture.
    pub const fn to_panel<PANEL: EpdPanel>(self, x: u32, y: u32) -> Option<(u32, u32)> {
        let (w, h) = self.size::<PANEL>();
        if x >= w || y >= h {
            return None;
        }
        Some(match self {
            Self::Deg0 => (x, y),
            Self::Deg90 => (PANEL::WIDTH - 1 - y, x),
            Self::Deg180 => (PANEL::WIDTH - 1 - x, PANEL::HEIGHT - 1 - y),
            Self::Deg270 => (y, PANEL::HEIGHT - 1 - x),
        })
    }

    /// Maps a picture rectangle to the panel rectangle it covers, clipped to the picture.
    /// Empty when nothing is left.
    pub fn rect_to_panel<PANEL: EpdPanel>(self, rect: Rect) -> Rect {
        let (w, h) = self.size::<PANEL>();
        let x1 = rect.x.saturating_add(rect.w).min(w);
        let y1 = rect.y.saturating_add(rect.h).min(h);
        if rect.x >= x1 || rect.y >= y1 {
            return Rect::new(0, 0, 0, 0);
        }
        let (x0, y0, x1, y1) = (rect.x, rect.y, x1 - 1, y1 - 1);
        // The corners of a rectangle stay corners under a quarter turn, so two opposite ones
        // give the bounding box.
        match (
            self.to_panel::<PANEL>(x0, y0),
            self.to_panel::<PANEL>(x1, y1),
        ) {
            (Some((ax, ay)), Some((bx, by))) => {
                let (left, top) = (ax.min(bx), ay.min(by));
                Rect::new(left, top, ax.max(bx) - left + 1, ay.max(by) - top + 1)
            }
            _ => Rect::new(0, 0, 0, 0),
        }
    }
}
