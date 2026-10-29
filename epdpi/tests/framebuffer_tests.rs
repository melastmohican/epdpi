#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use epdpi::prelude::*;

type P = Ed047tc1;

#[test]
fn rejects_undersized_buffer() {
    let mut small = vec![0u8; P::FB_BYTES - 1];
    let err = EpdFrameBuffer::<P>::new(&mut small).err();
    assert_eq!(
        err,
        Some(ValidationError::BufferTooSmall {
            required: P::FB_BYTES,
            provided: P::FB_BYTES - 1
        })
    );
}

#[test]
fn nibble_order_is_left_pixel_high() {
    let mut buf = vec![0u8; P::FB_BYTES];
    let mut fb = EpdFrameBuffer::<P>::new(&mut buf).unwrap();
    fb.set_pixel(0, 0, 0xA);
    fb.set_pixel(1, 0, 0x5);
    fb.set_pixel(2, 1, 0x3);
    assert_eq!(fb.as_bytes()[0], 0xA5);
    assert_eq!(fb.as_bytes()[P::FB_STRIDE_BYTES + 1], 0x30);
    assert_eq!(fb.pixel(0, 0), Some(0xA));
    assert_eq!(fb.pixel(1, 0), Some(0x5));
    assert_eq!(fb.pixel(959, 539), Some(0));
    assert_eq!(fb.pixel(960, 0), None);
}

#[test]
fn out_of_range_writes_are_ignored() {
    let mut buf = vec![0u8; P::FB_BYTES];
    let mut fb = EpdFrameBuffer::<P>::new(&mut buf).unwrap();
    fb.set_pixel(960, 0, 0xF);
    fb.set_pixel(0, 540, 0xF);
    assert!(fb.as_bytes().iter().all(|&b| b == 0));
}

#[test]
fn fill_sets_both_nibbles() {
    let mut buf = vec![0u8; P::FB_BYTES];
    let mut fb = EpdFrameBuffer::<P>::new(&mut buf).unwrap();
    fb.fill(0xF);
    assert!(fb.as_bytes().iter().all(|&b| b == 0xFF));
    fb.fill(0x3);
    assert!(fb.as_bytes().iter().all(|&b| b == 0x33));
}

#[cfg(feature = "graphics")]
#[test]
fn draw_target_clips_and_maps_luma() {
    use embedded_graphics::{
        pixelcolor::Gray4,
        prelude::*,
        primitives::{PrimitiveStyle, Rectangle},
    };
    let mut buf = vec![0xFFu8; P::FB_BYTES];
    let mut fb = EpdFrameBuffer::<P>::new(&mut buf).unwrap();
    assert_eq!(fb.size(), Size::new(960, 540));
    // Rectangle partly off-screen on the left and top.
    Rectangle::new(Point::new(-4, -4), Size::new(8, 8))
        .into_styled(PrimitiveStyle::with_fill(Gray4::new(2)))
        .draw(&mut fb)
        .unwrap();
    assert_eq!(fb.pixel(0, 0), Some(2));
    assert_eq!(fb.pixel(3, 3), Some(2));
    assert_eq!(fb.pixel(4, 4), Some(0xF));
}

#[test]
fn dirty_area_follows_writes_until_cleared() {
    let mut mem = vec![0xFFu8; P::FB_BYTES];
    let mut fb = EpdFrameBuffer::<P>::new(&mut mem).unwrap();
    // A new buffer has not been shown, so it is all dirty.
    assert_eq!(fb.dirty(), Some(Rect::full::<P>()));
    fb.clear_dirty();
    assert_eq!(fb.dirty(), None);
    // Pixels grow the bounding box.
    fb.set_pixel(10, 20, 0);
    assert_eq!(fb.dirty(), Some(Rect::new(10, 20, 1, 1)));
    fb.set_pixel(14, 25, 3);
    assert_eq!(fb.dirty(), Some(Rect::new(10, 20, 5, 6)));
    // Writes outside the panel change nothing.
    fb.clear_dirty();
    fb.set_pixel(960, 0, 1);
    fb.set_pixel(0, 540, 1);
    assert_eq!(fb.dirty(), None);
}

#[test]
fn fill_and_raw_access_mark_everything_dirty_and_mark_dirty_aligns() {
    let mut mem = vec![0xFFu8; P::FB_BYTES];
    let mut fb = EpdFrameBuffer::<P>::new(&mut mem).unwrap();
    fb.clear_dirty();
    fb.fill(0x3);
    assert_eq!(fb.dirty(), Some(Rect::full::<P>()));
    fb.clear_dirty();
    let _ = fb.as_bytes_mut();
    assert_eq!(fb.dirty(), Some(Rect::full::<P>()));
    fb.clear_dirty();
    // Reading marks nothing, mark_dirty widens to pixel pairs and ignores empty rectangles.
    let _ = fb.as_bytes();
    let _ = fb.pixel(5, 5);
    assert_eq!(fb.dirty(), None);
    fb.mark_dirty(Rect::new(3, 4, 4, 2));
    assert_eq!(fb.dirty(), Some(Rect::new(2, 4, 6, 2)));
    fb.clear_dirty();
    fb.mark_dirty(Rect::new(0, 0, 0, 0));
    assert_eq!(fb.dirty(), None);
}

#[cfg(feature = "graphics")]
#[test]
fn embedded_graphics_rectangles_convert_to_rects() {
    use embedded_graphics::{
        geometry::{Point, Size},
        primitives::Rectangle,
    };
    let rect = |x, y, w, h| Rect::from(Rectangle::new(Point::new(x, y), Size::new(w, h)));
    assert_eq!(rect(10, 20, 30, 40), Rect::new(10, 20, 30, 40));
    // A negative corner shrinks the rectangle from that side.
    assert_eq!(rect(-10, -5, 30, 20), Rect::new(0, 0, 20, 15));
    // No area, or wholly above and left of the origin: empty.
    assert!(rect(5, 5, 0, 10).is_empty());
    assert!(rect(5, 5, 10, 0).is_empty());
    assert!(rect(-30, 0, 20, 10).is_empty());
    assert!(rect(0, -30, 10, 20).is_empty());
    // Extreme sizes do not overflow.
    assert_eq!(
        rect(0, 0, u32::MAX, u32::MAX),
        Rect::new(0, 0, u32::MAX, u32::MAX)
    );
    assert_eq!(rect(i32::MIN, i32::MIN, u32::MAX, u32::MAX).x, 0);
}

#[cfg(feature = "graphics")]
#[test]
fn mark_dirty_takes_an_embedded_graphics_rectangle() {
    use embedded_graphics::{
        geometry::{Point, Size},
        primitives::Rectangle,
    };
    let mut mem = vec![0xFFu8; P::FB_BYTES];
    let mut fb = EpdFrameBuffer::<P>::new(&mut mem).unwrap();
    fb.clear_dirty();
    fb.mark_dirty(Rectangle::new(Point::new(-4, 10), Size::new(9, 3)));
    // x -4 clips to 0, width 5 widens to 6 (a whole pair).
    assert_eq!(fb.dirty(), Some(Rect::new(0, 10, 6, 3)));
    fb.clear_dirty();
    fb.mark_dirty(Rect::new(3, 4, 4, 2));
    assert_eq!(fb.dirty(), Some(Rect::new(2, 4, 6, 2)));
}
