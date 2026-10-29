//! The drawing surface: the 4 bits per pixel framebuffer, pixel rectangles and the dirty area.
//!
//! The types are always available. Only the `embedded-graphics` `DrawTarget` impl on
//! [`EpdFrameBuffer`] and the `From<Rectangle>` conversion on [`Rect`] need the `graphics`
//! feature.

pub mod framebuffer;
pub mod rect;

pub use framebuffer::EpdFrameBuffer;
pub use rect::Rect;
