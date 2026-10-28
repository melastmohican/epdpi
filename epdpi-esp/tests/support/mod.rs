//! Shared dual-mode test harness: recording pins and delays, and the
//! `epd_test!` macro so each test body is written once (async-first) and runs under both
//! `cargo test` (blocking) and `cargo test --no-default-features --features graphics` (async).
#![allow(dead_code)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use core::cell::RefCell;
use core::convert::Infallible;

#[cfg(feature = "blocking")]
use embedded_hal::delay::DelayNs;
#[cfg(not(feature = "blocking"))]
use embedded_hal_async::delay::DelayNs;

use embedded_hal::digital::{ErrorType, OutputPin};

use epdpi::prelude::*;

#[macro_export]
macro_rules! epd_test {
    ($name:ident, $body:ident) => {
        #[test]
        fn $name() {
            #[cfg(feature = "blocking")]
            {
                $body();
            }
            #[cfg(not(feature = "blocking"))]
            {
                pollster::block_on($body());
            }
        }
    };
}

/// Output pin that appends `(pin id, level)` to a shared log.
#[derive(Debug)]
pub struct RecordingPin<'a> {
    pub id: u8,
    pub log: &'a RefCell<Vec<(u8, bool)>>,
}

impl ErrorType for RecordingPin<'_> {
    type Error = Infallible;
}

impl OutputPin for RecordingPin<'_> {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.log.borrow_mut().push((self.id, false));
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.log.borrow_mut().push((self.id, true));
        Ok(())
    }
}

pub const D0: u8 = 0;
pub const CKV: u8 = 8;
pub const SPV: u8 = 9;
pub const SPH: u8 = 10;
pub const OE: u8 = 11;
pub const LE: u8 = 12;
pub const CL: u8 = 13;
pub const PWR: u8 = 14;

/// Records every delay in microseconds, with ms calls converted.
#[derive(Debug, Default)]
pub struct RecordingDelay<'a> {
    pub log: Option<&'a RefCell<Vec<(u8, bool)>>>,
    pub total_us: u64,
}

#[maybe_async_cfg::maybe(
    sync(feature = "blocking", keep_self),
    async(not(feature = "blocking"), keep_self)
)]
impl DelayNs for RecordingDelay<'_> {
    async fn delay_ns(&mut self, ns: u32) {
        self.total_us += u64::from(ns) / 1000;
    }
    async fn delay_us(&mut self, us: u32) {
        self.total_us += u64::from(us);
    }
    async fn delay_ms(&mut self, ms: u32) {
        self.total_us += u64::from(ms) * 1000;
    }
}

pub fn gpio_bus<'a>(
    log: &'a RefCell<Vec<(u8, bool)>>,
) -> GenericGpioBus<RecordingPin<'a>, RecordingDelay<'a>> {
    let pin = |id| RecordingPin { id, log };
    GenericGpioBus::new(
        core::array::from_fn(|i| pin(D0 + i as u8)),
        ControlPins {
            ckv: pin(CKV),
            spv: pin(SPV),
            sph: pin(SPH),
            oe: pin(OE),
            le: pin(LE),
            cl: pin(CL),
            pwr: pin(PWR),
        },
        RecordingDelay::default(),
    )
}
