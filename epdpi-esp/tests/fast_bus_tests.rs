#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! `FastGpioBus` mask table and line framing against a simulated GPIO port, run in both API
//! modes.

use core::cell::RefCell;

use epdpi::prelude::*;
use epdpi_esp::fast_bus::{FastGpioBus, GpioPort, PinMapError};

mod support;
use support::*;

/// The PaperDIY map: D0..D7 then CL.
const DATA_GPIO: [u8; 8] = [6, 14, 7, 12, 9, 11, 8, 10];
const CL_GPIO: u8 = 16;

/// Output register simulation. Records the data lines at every CL rising edge.
#[derive(Debug, Default)]
struct SimPort {
    state: u32,
    sampled: Vec<u8>,
}

impl SimPort {
    fn data_byte(&self) -> u8 {
        let mut byte = 0;
        for (bit, gpio) in DATA_GPIO.iter().enumerate() {
            if self.state & (1 << gpio) != 0 {
                byte |= 1 << bit;
            }
        }
        byte
    }
}

impl GpioPort for SimPort {
    fn set(&mut self, mask: u32) {
        let rising = mask & (1 << CL_GPIO) != 0 && self.state & (1 << CL_GPIO) == 0;
        self.state |= mask;
        if rising {
            let byte = self.data_byte();
            self.sampled.push(byte);
        }
    }

    fn clear(&mut self, mask: u32) {
        self.state &= !mask;
    }
}

fn fast_bus<'a>(
    log: &'a RefCell<Vec<(u8, bool)>>,
    data_gpio: [u8; 8],
    cl_gpio: u8,
) -> Result<FastGpioBus<RecordingPin<'a>, RecordingDelay<'a>, SimPort>, PinMapError> {
    let pin = |id| RecordingPin { id, log };
    FastGpioBus::with_port(
        SimPort::default(),
        core::array::from_fn(|i| pin(D0 + i as u8)),
        data_gpio,
        ControlPins {
            ckv: pin(CKV),
            spv: pin(SPV),
            sph: pin(SPH),
            oe: pin(OE),
            le: pin(LE),
            cl: pin(CL),
            pwr: pin(PWR),
        },
        cl_gpio,
        RecordingDelay::default(),
    )
}

#[test]
fn rejects_pins_outside_the_first_gpio_register() {
    let log = RefCell::new(Vec::new());
    let mut high_data = DATA_GPIO;
    high_data[3] = 32;
    assert_eq!(
        fast_bus(&log, high_data, CL_GPIO).err(),
        Some(PinMapError::PinOutOfRange)
    );
    assert_eq!(
        fast_bus(&log, DATA_GPIO, 45).err(),
        Some(PinMapError::PinOutOfRange)
    );
}

#[test]
fn rejects_duplicate_gpio_numbers() {
    let log = RefCell::new(Vec::new());
    let mut dup = DATA_GPIO;
    dup[1] = dup[0];
    assert_eq!(
        fast_bus(&log, dup, CL_GPIO).err(),
        Some(PinMapError::DuplicatePin)
    );
    assert_eq!(
        fast_bus(&log, DATA_GPIO, DATA_GPIO[4]).err(),
        Some(PinMapError::DuplicatePin)
    );
}

#[maybe_async_cfg::maybe(
    sync(feature = "blocking", keep_self),
    async(not(feature = "blocking"), keep_self)
)]
async fn every_byte_value_reaches_the_data_lines_at_the_clock_edge_body() {
    let log = RefCell::new(Vec::new());
    let mut bus = fast_bus(&log, DATA_GPIO, CL_GPIO).unwrap();
    let all: Vec<u8> = (0..=255).collect();
    bus.write_scanline(&all).await.unwrap();
    let (_, _, _, port) = bus.release();
    assert_eq!(port.sampled, all);
    // The clock ends low and only the nine fast pins were ever touched.
    assert_eq!(port.state & (1 << CL_GPIO), 0);
    let fast_pins = DATA_GPIO.iter().fold(1u32 << CL_GPIO, |m, g| m | (1 << g));
    assert_eq!(port.state & !fast_pins, 0);
}

epd_test!(
    every_byte_value_reaches_the_data_lines_at_the_clock_edge,
    every_byte_value_reaches_the_data_lines_at_the_clock_edge_body
);

#[maybe_async_cfg::maybe(
    sync(feature = "blocking", keep_self),
    async(not(feature = "blocking"), keep_self)
)]
async fn line_framing_matches_the_generic_bus_body() {
    let fast_log = RefCell::new(Vec::new());
    let mut fast = fast_bus(&fast_log, DATA_GPIO, CL_GPIO).unwrap();
    fast.write_scanline(&[0xA5, 0xFF]).await.unwrap();

    let generic_log = RefCell::new(Vec::new());
    let mut generic = gpio_bus(&generic_log);
    generic.write_scanline(&[0xA5, 0xFF]).await.unwrap();

    // The control pins see the same edges. The generic bus also toggles D0..D7 and CL.
    let control = |log: &RefCell<Vec<(u8, bool)>>| -> Vec<(u8, bool)> {
        log.borrow()
            .iter()
            .copied()
            .filter(|(pin, _)| matches!(*pin, LE | CKV | SPH))
            .collect()
    };
    assert_eq!(control(&fast_log), control(&generic_log));
    assert_eq!(
        control(&fast_log),
        vec![(LE, false), (CKV, true), (SPH, false), (SPH, true)]
    );
}

epd_test!(
    line_framing_matches_the_generic_bus,
    line_framing_matches_the_generic_bus_body
);

#[maybe_async_cfg::maybe(
    sync(feature = "blocking", keep_self),
    async(not(feature = "blocking"), keep_self)
)]
async fn control_sequences_are_delegated_to_the_generic_bus_body() {
    let log = RefCell::new(Vec::new());
    let mut bus = fast_bus(&log, DATA_GPIO, CL_GPIO).unwrap();
    bus.set_power(true).await.unwrap();
    bus.start_frame().await.unwrap();
    bus.next_row().await.unwrap();
    bus.end_frame().await.unwrap();
    let (_, _, delay, _) = bus.release();
    // Power on 1200 us, start of frame 23 us, same as the generic bus tests.
    assert_eq!(delay.total_us, 1200 + 23);
    assert_eq!(
        log.borrow()[..4],
        [(OE, true), (PWR, true), (SPV, true), (SPH, true)]
    );
}

epd_test!(
    control_sequences_are_delegated_to_the_generic_bus,
    control_sequences_are_delegated_to_the_generic_bus_body
);
