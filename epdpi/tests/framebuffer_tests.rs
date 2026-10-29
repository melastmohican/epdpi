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

#[test]
fn rotation_maps_corners_to_the_expected_panel_corners() {
    // (rotation, picture size, picture corners top-left, top-right, bottom-right, bottom-left)
    // and the panel corner each one lands on.
    let (w, h) = (P::WIDTH - 1, P::HEIGHT - 1);
    let cases = [
        (Rotation::Deg0, (960, 540), [(0, 0), (w, 0), (w, h), (0, h)]),
        (
            Rotation::Deg90,
            (540, 960),
            [(w, 0), (w, h), (0, h), (0, 0)],
        ),
        (
            Rotation::Deg180,
            (960, 540),
            [(w, h), (0, h), (0, 0), (w, 0)],
        ),
        (
            Rotation::Deg270,
            (540, 960),
            [(0, h), (0, 0), (w, 0), (w, h)],
        ),
    ];
    for (rotation, (pw, ph), expect) in cases {
        assert_eq!(rotation.size::<P>(), (pw, ph));
        let corners = [(0, 0), (pw - 1, 0), (pw - 1, ph - 1), (0, ph - 1)];
        for (corner, want) in corners.into_iter().zip(expect) {
            assert_eq!(
                rotation.to_panel::<P>(corner.0, corner.1),
                Some(want),
                "{rotation:?} {corner:?}"
            );
        }
        assert_eq!(rotation.to_panel::<P>(pw, 0), None);
        assert_eq!(rotation.to_panel::<P>(0, ph), None);
    }
}

#[test]
fn rotation_is_a_bijection_onto_the_panel() {
    let mut seen = vec![false; (P::WIDTH * P::HEIGHT) as usize];
    for rotation in [
        Rotation::Deg0,
        Rotation::Deg90,
        Rotation::Deg180,
        Rotation::Deg270,
    ] {
        seen.fill(false);
        let (w, h) = rotation.size::<P>();
        for y in 0..h {
            for x in 0..w {
                let (px, py) = rotation.to_panel::<P>(x, y).unwrap();
                let slot = &mut seen[(py * P::WIDTH + px) as usize];
                assert!(!*slot, "{rotation:?} maps two pixels to ({px}, {py})");
                *slot = true;
            }
        }
        assert!(
            seen.iter().all(|&s| s),
            "{rotation:?} leaves panel pixels unused"
        );
    }
}

#[test]
fn rotated_writes_land_in_panel_space_and_read_back() {
    let mut buf = vec![0xFFu8; P::FB_BYTES];
    let mut fb = EpdFrameBuffer::<P>::new(&mut buf).unwrap();
    assert_eq!(fb.rotation(), Rotation::Deg0);
    fb.set_rotation(Rotation::Deg90);
    assert_eq!(fb.logical_size(), (540, 960));
    fb.set_logical_pixel(10, 20, 0x3);
    // Deg90: panel x = WIDTH - 1 - y, panel y = x.
    assert_eq!(fb.pixel(959 - 20, 10), Some(0x3));
    assert_eq!(fb.logical_pixel(10, 20), Some(0x3));
    assert_eq!(fb.logical_pixel(540, 0), None);
    // Outside the picture is ignored, even where the panel has room.
    fb.set_logical_pixel(540, 0, 0x0);
    assert_eq!(fb.pixel(959, 540 - 1), Some(0xF));
    // Raw access is untouched by the rotation.
    fb.set_pixel(0, 0, 0x7);
    assert_eq!(fb.pixel(0, 0), Some(0x7));
}

#[test]
fn rotated_rectangles_cover_the_same_pixels() {
    let mut buf = vec![0xFFu8; P::FB_BYTES];
    let mut fb = EpdFrameBuffer::<P>::new(&mut buf).unwrap();
    for rotation in [
        Rotation::Deg0,
        Rotation::Deg90,
        Rotation::Deg180,
        Rotation::Deg270,
    ] {
        fb.set_rotation(rotation);
        let rect = Rect::new(30, 50, 7, 11);
        let panel = fb.rect_to_panel(rect);
        let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
        for y in rect.y..rect.y + rect.h {
            for x in rect.x..rect.x + rect.w {
                let (px, py) = rotation.to_panel::<P>(x, y).unwrap();
                (x0, y0, x1, y1) = (x0.min(px), y0.min(py), x1.max(px), y1.max(py));
            }
        }
        assert_eq!(
            panel,
            Rect::new(x0, y0, x1 - x0 + 1, y1 - y0 + 1),
            "{rotation:?}"
        );
    }
    // Clipped to the picture, and empty when wholly outside it.
    fb.set_rotation(Rotation::Deg90);
    assert_eq!(
        fb.rect_to_panel(Rect::new(500, 900, 100, 100)),
        fb.rect_to_panel(Rect::new(500, 900, 40, 60))
    );
    assert!(fb.rect_to_panel(Rect::new(540, 0, 5, 5)).is_empty());
}

#[cfg(feature = "graphics")]
#[test]
fn draw_target_follows_the_rotation() {
    use embedded_graphics::{pixelcolor::Gray4, prelude::*, primitives::Rectangle};
    let mut buf = vec![0xFFu8; P::FB_BYTES];
    let mut fb = EpdFrameBuffer::<P>::new(&mut buf).unwrap();
    fb.set_rotation(Rotation::Deg270);
    assert_eq!(fb.size(), Size::new(540, 960));
    // A 2 by 1 picture strip at the top left: Deg270 puts the top of the picture on the
    // left panel edge, and picture x runs up the panel.
    fb.fill_solid(
        &Rectangle::new(Point::new(0, 0), Size::new(2, 1)),
        Gray4::new(1),
    )
    .unwrap();
    assert_eq!(fb.pixel(0, 539), Some(1));
    assert_eq!(fb.pixel(0, 538), Some(1));
    assert_eq!(fb.pixel(0, 537), Some(0xF));
    assert_eq!(fb.pixel(1, 539), Some(0xF));
}
