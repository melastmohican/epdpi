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
