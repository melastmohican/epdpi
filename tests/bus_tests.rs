#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! `GenericGpioBus` pin-level behaviour, written once and run in both API modes.

use core::cell::RefCell;

use epdpi::prelude::*;

mod support;
use support::*;

#[maybe_async_cfg::maybe(
    sync(feature = "blocking", keep_self),
    async(not(feature = "blocking"), keep_self)
)]
async fn write_scanline_drives_data_pins_and_clocks_each_byte_body() {
    let log = RefCell::new(Vec::new());
    let mut bus = gpio_bus(&log);
    bus.write_scanline(&[0b1010_0101, 0xFF]).await.unwrap();
    let ev = log.borrow().clone();

    // Starts with SPH low and CKV high, ends with SPH high.
    assert_eq!(ev[0], (SPH, false));
    assert_eq!(ev[1], (CKV, true));
    assert_eq!(*ev.last().unwrap(), (SPH, true));

    let clocks: Vec<_> = ev.iter().filter(|(p, _)| *p == CL).collect();
    assert_eq!(clocks.len(), 4, "one rising and one falling edge per byte");
    assert_eq!(*clocks[0], (CL, true));
    assert_eq!(*clocks[1], (CL, false));

    // Data bits are settled before the first CL rising edge: bit n drives Dn.
    let first_clk = ev.iter().position(|e| *e == (CL, true)).unwrap();
    let mut levels = [false; 8];
    for &(pin, high) in &ev[..first_clk] {
        if pin < 8 {
            levels[pin as usize] = high;
        }
    }
    assert_eq!(levels, [true, false, true, false, false, true, false, true]);
}

epd_test!(
    write_scanline_drives_data_pins_and_clocks_each_byte,
    write_scanline_drives_data_pins_and_clocks_each_byte_body
);

#[maybe_async_cfg::maybe(
    sync(feature = "blocking", keep_self),
    async(not(feature = "blocking"), keep_self)
)]
async fn next_row_drops_ckv_then_pulses_latch_body() {
    let log = RefCell::new(Vec::new());
    let mut bus = gpio_bus(&log);
    bus.next_row().await.unwrap();
    assert_eq!(*log.borrow(), vec![(CKV, false), (LE, true), (LE, false)]);
}

epd_test!(
    next_row_drops_ckv_then_pulses_latch,
    next_row_drops_ckv_then_pulses_latch_body
);

#[maybe_async_cfg::maybe(
    sync(feature = "blocking", keep_self),
    async(not(feature = "blocking"), keep_self)
)]
async fn start_frame_pulses_spv_low_then_high_with_reference_timing_body() {
    let log = RefCell::new(Vec::new());
    let mut bus = gpio_bus(&log);
    bus.start_frame().await.unwrap();
    let ev = log.borrow().clone();
    let spv: Vec<bool> = ev.iter().filter(|(p, _)| *p == SPV).map(|e| e.1).collect();
    assert_eq!(spv, vec![false, true]);
    // 7 + 10 + 8 + 10 + 18 + 18 microseconds between edges.
    let (_, _, delay) = bus.release();
    assert_eq!(delay.total_us, 71);
    // CKV is left high, ready for the first scanline.
    assert_eq!(*ev.last().unwrap(), (CKV, true));
}

epd_test!(
    start_frame_pulses_spv_low_then_high_with_reference_timing,
    start_frame_pulses_spv_low_then_high_with_reference_timing_body
);

#[maybe_async_cfg::maybe(
    sync(feature = "blocking", keep_self),
    async(not(feature = "blocking"), keep_self)
)]
async fn power_sequence_orders_oe_pwr_spv_body() {
    let log = RefCell::new(Vec::new());
    let mut bus = gpio_bus(&log);
    bus.set_power(true).await.unwrap();
    assert_eq!(
        *log.borrow(),
        vec![(OE, true), (PWR, true), (SPV, true), (SPH, true)]
    );
    log.borrow_mut().clear();
    bus.set_power(false).await.unwrap();
    assert_eq!(*log.borrow(), vec![(PWR, false), (OE, false), (SPV, false)]);
}

epd_test!(
    power_sequence_orders_oe_pwr_spv,
    power_sequence_orders_oe_pwr_spv_body
);
