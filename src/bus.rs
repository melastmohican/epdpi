//! Portable bit-banged parallel bus built on `embedded-hal` 1.0 pins.

#[cfg(feature = "blocking")]
use embedded_hal::delay::DelayNs;
#[cfg(not(feature = "blocking"))]
use embedded_hal_async::delay::DelayNs;

use embedded_hal::digital::OutputPin;

use crate::traits::EpdParallelBus;

/// The seven control lines of the parallel interface.
#[derive(Debug)]
pub struct ControlPins<P> {
    /// Gate clock, shifts the active row.
    pub ckv: P,
    /// Gate start pulse, marks the first row of a frame.
    pub spv: P,
    /// Source start pulse, low while a scanline is clocked in.
    pub sph: P,
    /// Source output enable.
    pub oe: P,
    /// Source latch enable.
    pub le: P,
    /// Source clock, one rising edge per data byte.
    pub cl: P,
    /// Panel power rail.
    pub pwr: P,
}

/// Bit-banged [`EpdParallelBus`] that works on any MCU with enough GPIOs.
///
/// All fifteen pins share one type `P`, which is how `esp-hal` (`Output`), `embassy-rp`
/// and `embassy-stm32` expose their pins. Use `degrade`, `AnyPin` or a similar erasing
/// step first on HALs with a distinct type per pin.
///
/// The edge sequences and microsecond delays follow M5GFX's `Bus_EPD`, which is the sequence
/// confirmed on a PaperDIY. An earlier FastEPD-style version (short LE pulse, no wait after
/// power-on) left noise on the panel. Which of those differences mattered was not isolated.
#[derive(Debug)]
pub struct GenericGpioBus<P, D> {
    pub(crate) data: [P; 8],
    pub(crate) ctl: ControlPins<P>,
    pub(crate) delay: D,
}

impl<P, D> GenericGpioBus<P, D> {
    /// Wraps the data pins (D0 first) and control pins. The pins are assumed to be outputs
    /// already.
    pub fn new(data: [P; 8], control: ControlPins<P>, delay: D) -> Self {
        Self {
            data,
            ctl: control,
            delay,
        }
    }

    /// Gives the pins and delay back.
    pub fn release(self) -> ([P; 8], ControlPins<P>, D) {
        (self.data, self.ctl, self.delay)
    }
}

fn level<P: OutputPin>(pin: &mut P, high: bool) -> Result<(), P::Error> {
    if high { pin.set_high() } else { pin.set_low() }
}

#[maybe_async_cfg::maybe(
    sync(feature = "blocking", keep_self),
    async(not(feature = "blocking"), keep_self)
)]
impl<P, D> EpdParallelBus for GenericGpioBus<P, D>
where
    P: OutputPin,
    D: DelayNs,
{
    type Error = P::Error;

    async fn set_power(&mut self, on: bool) -> Result<(), Self::Error> {
        if on {
            self.ctl.oe.set_high()?;
            self.delay.delay_us(100).await;
            self.ctl.pwr.set_high()?;
            self.delay.delay_us(100).await;
            self.ctl.spv.set_high()?;
            self.ctl.sph.set_high()?;
            // The boost converter needs a moment before the first scan.
            self.delay.delay_ms(1).await;
        } else {
            self.delay.delay_ms(1).await;
            self.ctl.pwr.set_low()?;
            self.delay.delay_us(10).await;
            self.ctl.oe.set_low()?;
            self.delay.delay_us(100).await;
            self.ctl.spv.set_low()?;
        }
        Ok(())
    }

    async fn start_frame(&mut self) -> Result<(), Self::Error> {
        let c = &mut self.ctl;
        c.spv.set_low()?;
        self.delay.delay_us(1).await;
        c.ckv.set_low()?;
        self.delay.delay_us(3).await;
        c.ckv.set_high()?;
        self.delay.delay_us(1).await;
        c.spv.set_high()?;
        for _ in 0..3 {
            self.delay.delay_us(3).await;
            c.ckv.set_low()?;
            self.delay.delay_us(3).await;
            c.ckv.set_high()?;
        }
        Ok(())
    }

    async fn write_scanline(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        // LE stays high between rows and drops before the next row's data, as M5GFX does.
        self.ctl.le.set_low()?;
        self.ctl.ckv.set_high()?;
        self.ctl.sph.set_low()?;
        for &byte in data {
            for (bit, pin) in self.data.iter_mut().enumerate() {
                level(pin, byte & (1 << bit) != 0)?;
            }
            self.ctl.cl.set_high()?;
            self.ctl.cl.set_low()?;
        }
        self.ctl.sph.set_high()?;
        Ok(())
    }

    async fn next_row(&mut self) -> Result<(), Self::Error> {
        self.ctl.ckv.set_low()?;
        self.ctl.le.set_high()?;
        Ok(())
    }

    async fn end_frame(&mut self) -> Result<(), Self::Error> {
        self.ctl.le.set_low()?;
        self.ctl.ckv.set_high()?;
        Ok(())
    }
}
