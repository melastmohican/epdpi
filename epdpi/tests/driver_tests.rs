#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! `EpdDisplay` sequencing against a recording bus, run in both API modes.

use epdpi::prelude::*;

mod support;
use support::*;

type P = Ed047tc1;
const ROWS: usize = P::HEIGHT as usize;

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
    assert_eq!(frames(&bus), 8 + 8 + 1);
    assert_eq!(
        bus.count(|e| matches!(e, BusEvent::Line(248, _))),
        17 * ROWS
    );
    assert_eq!(bus.count(|e| *e == BusEvent::NextRow), 17 * ROWS);
    // Darken pass, lighten pass, then a neutral discharge pass.
    assert!(bus.lines[0][..240].iter().all(|&b| b == 0x55));
    assert!(bus.lines[8 * ROWS][..240].iter().all(|&b| b == 0xAA));
    assert!(bus.lines[16 * ROWS].iter().all(|&b| b == 0));
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

    // 16 clear frames, 8 LUT passes, 1 discharge.
    assert_eq!(frames(&bus), 16 + 8 + 1);
    assert_eq!(bus.lines.len(), 25 * ROWS);
    // LUT passes start after the clear frames; black pixels darken on every pass.
    for pass in 0..8 {
        let first = &bus.lines[(16 + pass) * ROWS];
        assert!(first[..240].iter().all(|&b| b == 0x55), "pass {pass}");
        let last = &bus.lines[(16 + pass) * ROWS + ROWS - 1];
        assert!(last[..240].iter().all(|&b| b == 0x55), "pass {pass}");
    }
    assert!(bus.lines[24 * ROWS].iter().all(|&b| b == 0));
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
