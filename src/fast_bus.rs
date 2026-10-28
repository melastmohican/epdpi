//! Register-level bus for the ESP32-S3.
//!
//! [`GenericGpioBus`] sets each data pin with its own call, which takes about 250 us per
//! row. The ED047TC1 waveform matrix assumes a row time near 16 us (M5GFX), so on that bus
//! every pass drives too long and the mid grays saturate to two tones. [`FastGpioBus`]
//! sets all eight data lines and the clock with two stores to the GPIO set and clear
//! registers per byte, which lands close to the M5GFX row time.
//!
//! The crate has no HAL dependency, so the caller passes the pins and their GPIO numbers.
//! The pins are only held, to keep them configured as outputs. The data pins and CL must
//! all be below GPIO32, because the fast path uses the first GPIO output register.

use crate::bus::{ControlPins, GenericGpioBus};
use crate::traits::EpdParallelBus;

#[cfg(feature = "blocking")]
use embedded_hal::delay::DelayNs;
#[cfg(not(feature = "blocking"))]
use embedded_hal_async::delay::DelayNs;

use embedded_hal::digital::OutputPin;

/// Why a pin map was rejected by [`FastGpioBus::with_port`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum PinMapError {
    /// A data pin or CL is GPIO32 or higher, outside the first GPIO output register.
    PinOutOfRange,
    /// Two of the nine fast pins (D0 to D7, CL) share a GPIO number.
    DuplicatePin,
}

/// Write-one-to-set and write-one-to-clear access to the GPIO outputs below GPIO32.
pub trait GpioPort {
    /// Drives the pins in `mask` high, leaving the others alone.
    fn set(&mut self, mask: u32);
    /// Drives the pins in `mask` low, leaving the others alone.
    fn clear(&mut self, mask: u32);
}

/// The ESP32-S3 `GPIO_OUT_W1TS_REG` and `GPIO_OUT_W1TC_REG` registers (TRM, GPIO base
/// `0x6000_4000`).
#[derive(Debug)]
pub struct Esp32s3Port(());

const OUT_W1TS: *mut u32 = 0x6000_4008 as *mut u32;
const OUT_W1TC: *mut u32 = 0x6000_400C as *mut u32;

impl GpioPort for Esp32s3Port {
    fn set(&mut self, mask: u32) {
        // SAFETY: fixed ESP32-S3 register. A write only affects the bits set in `mask`, and
        // `Esp32s3Bus::new` only builds this port for pins the bus owns.
        unsafe { OUT_W1TS.write_volatile(mask) }
    }

    fn clear(&mut self, mask: u32) {
        // SAFETY: as for `set`.
        unsafe { OUT_W1TC.write_volatile(mask) }
    }
}

/// [`FastGpioBus`] on the ESP32-S3 GPIO registers.
pub type Esp32s3Bus<P, D> = FastGpioBus<P, D, Esp32s3Port>;

/// Bus that streams scanlines through a [`GpioPort`] and delegates the control sequences
/// to [`GenericGpioBus`], so the edge order is the one confirmed on hardware.
#[derive(Debug)]
pub struct FastGpioBus<P, D, G> {
    inner: GenericGpioBus<P, D>,
    port: G,
    /// GPIO bit pattern on D0..D7 for each byte value.
    table: [u32; 256],
    all_data: u32,
    cl: u32,
}

impl<P, D> Esp32s3Bus<P, D> {
    /// Wraps the data pins (D0 first) and control pins, which must already be outputs.
    ///
    /// `data_gpio` and `cl_gpio` are the GPIO numbers of the pins in `data` and
    /// `control.cl`. They are not checked against the pins, so a wrong number drives
    /// another pin. Returns an error if any of them is 32 or higher or if two are equal.
    pub fn new(
        data: [P; 8],
        data_gpio: [u8; 8],
        control: ControlPins<P>,
        cl_gpio: u8,
        delay: D,
    ) -> Result<Self, PinMapError> {
        Self::with_port(Esp32s3Port(()), data, data_gpio, control, cl_gpio, delay)
    }
}

impl<P, D, G> FastGpioBus<P, D, G> {
    /// Like [`Esp32s3Bus::new`] with a caller-supplied port, for tests or other chips whose
    /// GPIO registers work the same way.
    pub fn with_port(
        port: G,
        data: [P; 8],
        data_gpio: [u8; 8],
        control: ControlPins<P>,
        cl_gpio: u8,
        delay: D,
    ) -> Result<Self, PinMapError> {
        let mut seen = 0u32;
        for gpio in data_gpio.into_iter().chain([cl_gpio]) {
            if gpio >= 32 {
                return Err(PinMapError::PinOutOfRange);
            }
            let bit = 1u32 << gpio;
            if seen & bit != 0 {
                return Err(PinMapError::DuplicatePin);
            }
            seen |= bit;
        }
        let cl = 1u32 << cl_gpio;
        let all_data = seen & !cl;
        let mut table = [0u32; 256];
        for (value, slot) in table.iter_mut().enumerate() {
            for (bit, gpio) in data_gpio.iter().enumerate() {
                if value & (1 << bit) != 0 {
                    *slot |= 1 << gpio;
                }
            }
        }
        Ok(Self {
            inner: GenericGpioBus::new(data, control, delay),
            port,
            table,
            all_data,
            cl,
        })
    }

    /// Gives the pins, port and delay back.
    pub fn release(self) -> ([P; 8], ControlPins<P>, D, G) {
        let (data, control, delay) = self.inner.release();
        (data, control, delay, self.port)
    }
}

#[maybe_async_cfg::maybe(
    sync(feature = "blocking", keep_self),
    async(not(feature = "blocking"), keep_self)
)]
impl<P, D, G> EpdParallelBus for FastGpioBus<P, D, G>
where
    P: OutputPin,
    D: DelayNs,
    G: GpioPort,
{
    type Error = P::Error;

    async fn set_power(&mut self, on: bool) -> Result<(), Self::Error> {
        self.inner.set_power(on).await
    }

    async fn start_frame(&mut self) -> Result<(), Self::Error> {
        self.inner.start_frame().await
    }

    async fn write_scanline(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        // Same line framing as `GenericGpioBus`, with the per-byte pin writes replaced by
        // two stores for the data lines and one each to raise and drop CL.
        self.inner.ctl.le.set_low()?;
        self.inner.ctl.ckv.set_high()?;
        self.inner.ctl.sph.set_low()?;
        for &byte in data {
            let ones = self.table[usize::from(byte)];
            self.port.clear(self.all_data & !ones);
            self.port.set(ones);
            self.port.set(self.cl);
            self.port.clear(self.cl);
        }
        self.inner.ctl.sph.set_high()?;
        Ok(())
    }

    async fn next_row(&mut self) -> Result<(), Self::Error> {
        self.inner.next_row().await
    }

    async fn end_frame(&mut self) -> Result<(), Self::Error> {
        self.inner.end_frame().await
    }
}
