#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! `EpdDisplay` sequencing against a recording bus, run in both API modes.

use epdpi::prelude::*;

mod support;
use support::*;

type P = Ed047tc1;
const ROWS: usize = P::HEIGHT as usize;
/// Darken passes, and lighten passes, in a clear cycle (`CLEAR_PASSES` in the driver).
const CLEAR_PASSES: usize = 16;

fn frames(bus: &RecordingBus) -> usize {
    bus.count(|e| *e == BusEvent::StartFrame)
}

#[test]
fn builder_rejects_short_buffers() {
    let mut prev = vec![0u8; P::FB_BYTES - 1];
    let mut line = vec![0u8; P::SCANLINE_BYTES];
    let r = EpdDisplayBuilder::<P>::new().build(RecordingBus::default(), &mut prev, &mut line);
    assert!(matches!(r, Err(ValidationError::BufferTooSmall { .. })));

    let mut prev = vec![0u8; P::FB_BYTES];
    let mut line = vec![0u8; P::SCANLINE_BYTES - 1];
    let r = EpdDisplayBuilder::<P>::new().build(RecordingBus::default(), &mut prev, &mut line);
    assert!(matches!(
        r,
        Err(ValidationError::BufferTooSmall {
            required: 248,
            provided: 247
        })
    ));
}

#[test]
fn vcom_defaults_to_panel_and_can_be_overridden() {
    let mut prev = vec![0u8; P::FB_BYTES];
    let mut line = vec![0u8; P::SCANLINE_BYTES];
    let d = EpdDisplayBuilder::<P>::new()
        .build(RecordingBus::default(), &mut prev, &mut line)
        .unwrap();
    assert_eq!(d.vcom_mv(), -2000);
    drop(d);
    let d = EpdDisplayBuilder::<P>::new()
        .with_vcom(-2150)
        .build(RecordingBus::default(), &mut prev, &mut line)
        .unwrap();
    assert_eq!(d.vcom_mv(), -2150);
}

#[maybe_async_cfg::maybe(
    sync(feature = "blocking", keep_self),
    async(not(feature = "blocking"), keep_self)
)]
async fn clear_flashes_black_then_white_then_discharges_body() {
    let mut prev = vec![0u8; P::FB_BYTES];
    let mut line = vec![0u8; P::SCANLINE_BYTES];
    let mut d = EpdDisplayBuilder::<P>::new()
        .build(RecordingBus::default(), &mut prev, &mut line)
        .unwrap();
    d.clear().await.unwrap();
    let bus = d.release();

    assert_eq!(bus.events.first(), Some(&BusEvent::Power(true)));
    assert_eq!(bus.events.last(), Some(&BusEvent::Power(false)));
    assert_eq!(frames(&bus), 2 * CLEAR_PASSES + 1);
    assert_eq!(
        bus.count(|e| matches!(e, BusEvent::Line(248, _))),
        (2 * CLEAR_PASSES + 1) * ROWS
    );
    assert_eq!(
        bus.count(|e| *e == BusEvent::NextRow),
        (2 * CLEAR_PASSES + 1) * ROWS
    );
    // Darken pass, lighten pass, then a neutral discharge pass.
    assert!(bus.lines[0][..240].iter().all(|&b| b == 0x55));
    assert!(
        bus.lines[CLEAR_PASSES * ROWS][..240]
            .iter()
            .all(|&b| b == 0xAA)
    );
    assert!(bus.lines[2 * CLEAR_PASSES * ROWS].iter().all(|&b| b == 0));
    // Every scanline carries the 8 padding bytes as zeros.
    assert!(bus.lines.iter().all(|l| l[240..].iter().all(|&b| b == 0)));
    // The record of the glass is now white.
    drop(bus);
    assert!(prev.iter().all(|&b| b == 0xFF));
}

epd_test!(
    clear_flashes_black_then_white_then_discharges,
    clear_flashes_black_then_white_then_discharges_body
);

#[maybe_async_cfg::maybe(
    sync(feature = "blocking", keep_self),
    async(not(feature = "blocking"), keep_self)
)]
async fn quality16_clears_then_runs_every_lut_pass_body() {
    let mut prev = vec![0u8; P::FB_BYTES];
    let mut line = vec![0u8; P::SCANLINE_BYTES];
    let mut fbmem = vec![0u8; P::FB_BYTES];
    let mut fb = EpdFrameBuffer::<P>::new(&mut fbmem).unwrap();
    fb.fill(0); // all black
    let mut d = EpdDisplayBuilder::<P>::new()
        .build(RecordingBus::default(), &mut prev, &mut line)
        .unwrap();
    d.refresh(&fb, RefreshMode::Quality16).await.unwrap();
    let bus = d.release();

    // The clear frames, 8 LUT passes, 1 discharge.
    let clear = 2 * CLEAR_PASSES;
    assert_eq!(frames(&bus), clear + 8 + 1);
    assert_eq!(bus.lines.len(), (clear + 9) * ROWS);
    // LUT passes start after the clear frames; black pixels darken on every pass.
    for pass in 0..8 {
        let first = &bus.lines[(clear + pass) * ROWS];
        assert!(first[..240].iter().all(|&b| b == 0x55), "pass {pass}");
        let last = &bus.lines[(clear + pass) * ROWS + ROWS - 1];
        assert!(last[..240].iter().all(|&b| b == 0x55), "pass {pass}");
    }
    assert!(bus.lines[(clear + 8) * ROWS].iter().all(|&b| b == 0));
    drop(bus);
    assert!(
        prev.iter().all(|&b| b == 0x00),
        "prev records the shown frame"
    );
}

epd_test!(
    quality16_clears_then_runs_every_lut_pass,
    quality16_clears_then_runs_every_lut_pass_body
);

#[maybe_async_cfg::maybe(
    sync(feature = "blocking", keep_self),
    async(not(feature = "blocking"), keep_self)
)]
async fn fast_mono_only_drives_changed_pixels_body() {
    let mut prev = vec![0xFFu8; P::FB_BYTES]; // glass is white
    let mut line = vec![0u8; P::SCANLINE_BYTES];
    let mut fbmem = vec![0xFFu8; P::FB_BYTES];
    let mut fb = EpdFrameBuffer::<P>::new(&mut fbmem).unwrap();
    fb.set_pixel(0, 2, 0); // one black pixel on row 2
    let mut d = EpdDisplayBuilder::<P>::new()
        .build(RecordingBus::default(), &mut prev, &mut line)
        .unwrap();
    d.refresh(&fb, RefreshMode::FastMono).await.unwrap();
    let bus = d.release();

    // 4 mono passes plus the discharge pass, no clear flash.
    assert_eq!(frames(&bus), 4 + 1);
    for pass in 0..4 {
        for row in 0..ROWS {
            let l = &bus.lines[pass * ROWS + row];
            if row == 2 {
                assert_eq!(l[0], 0b01_00_00_00, "pass {pass}");
                assert!(l[1..].iter().all(|&b| b == 0));
            } else {
                assert!(l.iter().all(|&b| b == 0), "pass {pass} row {row}");
            }
        }
    }
    drop(bus);
    // A second identical refresh drives nothing.
    let mut d = EpdDisplayBuilder::<P>::new()
        .build(RecordingBus::default(), &mut prev, &mut line)
        .unwrap();
    d.refresh(&fb, RefreshMode::FastMono).await.unwrap();
    let bus = d.release();
    assert!(bus.lines.iter().all(|l| l.iter().all(|&b| b == 0)));
}

epd_test!(
    fast_mono_only_drives_changed_pixels,
    fast_mono_only_drives_changed_pixels_body
);

#[maybe_async_cfg::maybe(
    sync(feature = "blocking", keep_self),
    async(not(feature = "blocking"), keep_self)
)]
async fn m5_quality_erases_from_the_record_then_draws_the_target_body() {
    let mut prev = vec![0x00u8; P::FB_BYTES]; // glass shows black
    let mut line = vec![0u8; P::SCANLINE_BYTES];
    let mut fbmem = vec![0xFFu8; P::FB_BYTES];
    let mut fb = EpdFrameBuffer::<P>::new(&mut fbmem).unwrap();
    fb.fill(0xF); // target all white
    let mut d = EpdDisplayBuilder::<P>::new()
        .build(RecordingBus::default(), &mut prev, &mut line)
        .unwrap();
    d.refresh(&fb, RefreshMode::M5Quality).await.unwrap();
    let bus = d.release();

    // No clear cycle: 2 erase passes, 15 drawing passes, 1 discharge.
    assert_eq!(frames(&bus), 2 + 15 + 1);
    assert_eq!(bus.lines.len(), 18 * ROWS);
    // Erase passes follow the recorded black: both lighten.
    for pass in 0..2 {
        let first = &bus.lines[pass * ROWS];
        assert!(first[..240].iter().all(|&b| b == 0xAA), "erase {pass}");
    }
    // Drawing passes follow the white target, row 15 of the quality table.
    for pass in 0..15 {
        let code = M5_QUALITY_LUT[15 * 15 + pass];
        let first = &bus.lines[(2 + pass) * ROWS];
        assert!(
            first[..240].iter().all(|&b| b == code * 0x55),
            "draw {pass}"
        );
    }
    assert!(bus.lines[17 * ROWS].iter().all(|&b| b == 0));
    drop(bus);
    assert!(
        prev.iter().all(|&b| b == 0xFF),
        "prev records the shown frame"
    );
}

epd_test!(
    m5_quality_erases_from_the_record_then_draws_the_target,
    m5_quality_erases_from_the_record_then_draws_the_target_body
);

#[maybe_async_cfg::maybe(
    sync(feature = "blocking", keep_self),
    async(not(feature = "blocking"), keep_self)
)]
async fn m5_fast_records_the_black_and_white_picture_body() {
    let mut prev = vec![0x00u8; P::FB_BYTES]; // glass shows black
    let mut line = vec![0u8; P::SCANLINE_BYTES];
    let mut fbmem = vec![0xFFu8; P::FB_BYTES];
    let mut fb = EpdFrameBuffer::<P>::new(&mut fbmem).unwrap();
    fb.fill(0x9); // gray 9 everywhere, counts as white
    let mut d = EpdDisplayBuilder::<P>::new()
        .build(RecordingBus::default(), &mut prev, &mut line)
        .unwrap();
    d.refresh(&fb, RefreshMode::M5Fast).await.unwrap();
    let bus = d.release();
    // 8 passes and the discharge pass, no clear cycle.
    assert_eq!(frames(&bus), 8 + 1);
    assert_eq!(bus.lines.len(), 9 * ROWS);
    // Black to white: first pass is the white row of `lut_fast`, a darken kick.
    assert!(bus.lines[0][..240].iter().all(|&b| b == 0x55));
    drop(bus);
    assert!(
        prev.iter().all(|&b| b == 0xFF),
        "record holds the reduced picture, not gray 9"
    );
}

epd_test!(
    m5_fast_records_the_black_and_white_picture,
    m5_fast_records_the_black_and_white_picture_body
);

#[maybe_async_cfg::maybe(
    sync(feature = "blocking", keep_self),
    async(not(feature = "blocking"), keep_self)
)]
async fn m5_modes_run_every_pass_of_their_own_table_body() {
    // Black on screen, white requested, so every pair is active in every mode.
    for (mode, erase, lut, passes) in [
        (RefreshMode::M5Fast, false, &M5_FAST_LUT[..], P::FAST_PASSES),
        (
            RefreshMode::M5Fastest,
            false,
            &M5_FASTEST_LUT[..],
            P::FASTEST_PASSES,
        ),
        (RefreshMode::M5Text, true, &M5_TEXT_LUT[..], P::TEXT_PASSES),
    ] {
        let mut prev = vec![0x00u8; P::FB_BYTES];
        let mut line = vec![0u8; P::SCANLINE_BYTES];
        let mut fbmem = vec![0xFFu8; P::FB_BYTES];
        let mut fb = EpdFrameBuffer::<P>::new(&mut fbmem).unwrap();
        fb.fill(0xF);
        let mut d = EpdDisplayBuilder::<P>::new()
            .build(RecordingBus::default(), &mut prev, &mut line)
            .unwrap();
        d.refresh(&fb, mode).await.unwrap();
        let bus = d.release();
        let erase_passes = if erase { P::ERASER_PASSES } else { 0 };
        let total = erase_passes + passes;
        assert_eq!(frames(&bus), total + 1, "{mode:?}");
        for pass in 0..total {
            let code = if pass < erase_passes {
                // Erase follows the recorded black, level 0 of the eraser table.
                M5_ERASER_LUT[pass]
            } else {
                // Drawing follows the white target, level 15.
                lut[15 * passes + pass - erase_passes]
            };
            let first = &bus.lines[pass * ROWS];
            assert!(
                first[..240].iter().all(|&b| b == code * 0x55),
                "{mode:?} pass {pass}"
            );
        }
    }
}

epd_test!(
    m5_modes_run_every_pass_of_their_own_table,
    m5_modes_run_every_pass_of_their_own_table_body
);

#[maybe_async_cfg::maybe(
    sync(feature = "blocking", keep_self),
    async(not(feature = "blocking"), keep_self)
)]
async fn refresh_rect_drives_only_the_window_and_records_only_the_window_body() {
    let mut prev = vec![0xFFu8; P::FB_BYTES]; // glass shows white
    let mut line = vec![0u8; P::SCANLINE_BYTES];
    let mut fbmem = vec![0xFFu8; P::FB_BYTES];
    let mut fb = EpdFrameBuffer::<P>::new(&mut fbmem).unwrap();
    // A black block inside the window and one outside it.
    for y in 105..110 {
        for x in 110..120 {
            fb.set_pixel(x, y, 0);
        }
    }
    // Outside in both directions, outside in columns only (rows 105..110 are inside the
    // window), and outside in rows only (columns 110..120 are inside).
    for (xs, ys) in [
        (400..420, 300..320),
        (400..420, 105..110),
        (110..120, 300..305),
    ] {
        for y in ys {
            for x in xs.clone() {
                fb.set_pixel(x, y, 0);
            }
        }
    }
    let mut d = EpdDisplayBuilder::<P>::new()
        .build(RecordingBus::default(), &mut prev, &mut line)
        .unwrap();
    // x 101 widens to 100, the right edge 140 stays.
    d.refresh_rect(&fb, RefreshMode::M5Fast, Rect::new(101, 100, 39, 20))
        .await
        .unwrap();
    let bus = d.release();
    assert_eq!(frames(&bus), 8 + 1);
    let mut driven = false;
    for pass in 0..8 {
        for row in 0..ROWS {
            let l = &bus.lines[pass * ROWS + row];
            let in_rows = (100..120).contains(&row);
            for (i, &b) in l[..240].iter().enumerate() {
                // Window columns 100..140 are scanline bytes 25..35.
                if !in_rows || !(25..35).contains(&i) {
                    assert_eq!(b, 0, "pass {pass} row {row} byte {i} must hold");
                } else if b != 0 {
                    driven = true;
                }
            }
        }
    }
    assert!(driven, "the block inside the window is driven");
    drop(bus);
    // Record: the block inside is stored, the one outside is not.
    let stride = P::FB_STRIDE_BYTES;
    assert_eq!(prev[105 * stride + 55], 0x00, "inside block recorded");
    assert_eq!(prev[300 * stride + 200], 0xFF, "outside block not recorded");
    assert_eq!(
        prev[105 * stride + 200],
        0xFF,
        "column-outside block not recorded"
    );
    assert_eq!(
        prev[300 * stride + 55],
        0xFF,
        "row-outside block not recorded"
    );
}

epd_test!(
    refresh_rect_drives_only_the_window_and_records_only_the_window,
    refresh_rect_drives_only_the_window_and_records_only_the_window_body
);

#[maybe_async_cfg::maybe(
    sync(feature = "blocking", keep_self),
    async(not(feature = "blocking"), keep_self)
)]
async fn refresh_rect_with_nothing_to_draw_does_nothing_body() {
    let mut prev = vec![0xFFu8; P::FB_BYTES];
    let mut line = vec![0u8; P::SCANLINE_BYTES];
    let mut fbmem = vec![0xFFu8; P::FB_BYTES];
    let fb = EpdFrameBuffer::<P>::new(&mut fbmem).unwrap();
    let mut d = EpdDisplayBuilder::<P>::new()
        .build(RecordingBus::default(), &mut prev, &mut line)
        .unwrap();
    d.refresh_rect(&fb, RefreshMode::M5Fast, Rect::new(0, 0, 0, 10))
        .await
        .unwrap();
    d.refresh_rect(&fb, RefreshMode::M5Fast, Rect::new(2000, 0, 10, 10))
        .await
        .unwrap();
    let bus = d.release();
    assert!(bus.events.is_empty(), "no power cycle, no frames");
}

epd_test!(
    refresh_rect_with_nothing_to_draw_does_nothing,
    refresh_rect_with_nothing_to_draw_does_nothing_body
);

#[maybe_async_cfg::maybe(
    sync(feature = "blocking", keep_self),
    async(not(feature = "blocking"), keep_self)
)]
async fn quality16_lines(windowed: bool) -> Vec<Vec<u8>> {
    let mut prev = vec![0xFFu8; P::FB_BYTES];
    let mut line = vec![0u8; P::SCANLINE_BYTES];
    let mut fbmem = vec![0x00u8; P::FB_BYTES];
    let fb = EpdFrameBuffer::<P>::new(&mut fbmem).unwrap();
    let mut d = EpdDisplayBuilder::<P>::new()
        .build(RecordingBus::default(), &mut prev, &mut line)
        .unwrap();
    if windowed {
        d.refresh_rect(&fb, RefreshMode::Quality16, Rect::new(10, 10, 4, 4))
            .await
            .unwrap();
    } else {
        d.refresh(&fb, RefreshMode::Quality16).await.unwrap();
    }
    d.release().lines
}

#[maybe_async_cfg::maybe(
    sync(feature = "blocking", keep_self),
    async(not(feature = "blocking"), keep_self)
)]
async fn refresh_rect_on_full_panel_modes_covers_the_whole_panel_body() {
    let windowed = quality16_lines(true).await;
    let full = quality16_lines(false).await;
    assert_eq!(windowed, full);
}

epd_test!(
    refresh_rect_on_full_panel_modes_covers_the_whole_panel,
    refresh_rect_on_full_panel_modes_covers_the_whole_panel_body
);

#[maybe_async_cfg::maybe(
    sync(feature = "blocking", keep_self),
    async(not(feature = "blocking"), keep_self)
)]
async fn refresh_dirty_shows_what_was_drawn_then_forgets_it_body() {
    let mut prev = vec![0xFFu8; P::FB_BYTES];
    let mut line = vec![0u8; P::SCANLINE_BYTES];
    let mut fbmem = vec![0xFFu8; P::FB_BYTES];
    let mut fb = EpdFrameBuffer::<P>::new(&mut fbmem).unwrap();
    fb.clear_dirty();
    // Nothing drawn: nothing happens.
    let mut d = EpdDisplayBuilder::<P>::new()
        .build(RecordingBus::default(), &mut prev, &mut line)
        .unwrap();
    d.refresh_dirty(&mut fb, RefreshMode::M5Fast).await.unwrap();
    assert!(d.release().events.is_empty());

    // A block at x 110..120, y 105..110 is drawn, so only it is driven.
    for y in 105..110 {
        for x in 110..120 {
            fb.set_pixel(x, y, 0);
        }
    }
    let mut d = EpdDisplayBuilder::<P>::new()
        .build(RecordingBus::default(), &mut prev, &mut line)
        .unwrap();
    d.refresh_dirty(&mut fb, RefreshMode::M5Fast).await.unwrap();
    assert_eq!(fb.dirty(), None, "dirty area forgotten after a refresh");
    let bus = d.release();
    assert_eq!(frames(&bus), 8 + 1);
    for pass in 0..8 {
        for row in 0..ROWS {
            let l = &bus.lines[pass * ROWS + row];
            // Block columns 110..120 are scanline bytes 27..30.
            let inside = (105..110).contains(&row);
            for (i, &b) in l[..240].iter().enumerate() {
                if !inside || !(27..30).contains(&i) {
                    assert_eq!(b, 0, "pass {pass} row {row} byte {i}");
                }
            }
        }
    }
    drop(bus);
    assert_eq!(prev[105 * P::FB_STRIDE_BYTES + 55], 0x00);

    // A second call right away does nothing.
    let mut d = EpdDisplayBuilder::<P>::new()
        .build(RecordingBus::default(), &mut prev, &mut line)
        .unwrap();
    d.refresh_dirty(&mut fb, RefreshMode::M5Fast).await.unwrap();
    assert!(d.release().events.is_empty());
}

epd_test!(
    refresh_dirty_shows_what_was_drawn_then_forgets_it,
    refresh_dirty_shows_what_was_drawn_then_forgets_it_body
);
