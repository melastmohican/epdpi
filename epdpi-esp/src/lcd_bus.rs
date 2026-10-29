#![doc = include_str!("../docs/lcd_bus.md")]

#[cfg(feature = "blocking")]
use embedded_hal::delay::DelayNs;
#[cfg(not(feature = "blocking"))]
use embedded_hal_async::delay::DelayNs;
#[cfg(not(feature = "blocking"))]
use esp_hal::Async;
#[cfg(feature = "blocking")]
use esp_hal::Blocking;
use esp_hal::dma::{DmaError, DmaTxBuf};
use esp_hal::gpio::Output;
use esp_hal::lcd_cam::lcd::i8080::{Command, I8080};

use epdpi::EpdParallelBus;

/// Why a bus call failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LcdBusError {
    /// The DMA transfer failed.
    Dma(DmaError),
    /// A previous failure left the bus without its peripheral.
    Lost,
    /// The scanline does not fit the DMA buffer.
    TooLong,
}

#[cfg(feature = "blocking")]
type Mode = Blocking;
#[cfg(not(feature = "blocking"))]
type Mode = Async;

/// See the module docs. `D` times the settle delays and the frame start pulses.
pub struct LcdBus<D> {
    i8080: Option<I8080<'static, Mode>>,
    buf: Option<DmaTxBuf>,
    ckv: Output<'static>,
    spv: Output<'static>,
    oe: Output<'static>,
    le: Output<'static>,
    pwr: Output<'static>,
    delay: D,
}

/// The five control lines that stay plain GPIO. CL (write strobe) and SPH (chip select)
/// belong to the LCD peripheral, so they are not here.
#[derive(Debug)]
pub struct LcdControlPins<'d> {
    /// Gate clock. Held high for the length of each transfer.
    pub ckv: Output<'d>,
    /// Gate start pulse.
    pub spv: Output<'d>,
    /// Source output enable.
    pub oe: Output<'d>,
    /// Source latch enable.
    pub le: Output<'d>,
    /// Panel power rail.
    pub pwr: Output<'d>,
}

impl<D> LcdBus<D> {
    /// Wraps a configured `I8080` (data pins, write strobe and chip select attached), a DMA
    /// buffer at least one scanline long (`EpdPanel::SCANLINE_BYTES`), the control pins and a
    /// delay.
    pub fn new(
        i8080: I8080<'static, Mode>,
        buf: DmaTxBuf,
        control: LcdControlPins<'static>,
        delay: D,
    ) -> Self {
        let LcdControlPins {
            ckv,
            spv,
            oe,
            le,
            pwr,
        } = control;
        Self {
            i8080: Some(i8080),
            buf: Some(buf),
            ckv,
            spv,
            oe,
            le,
            pwr,
            delay,
        }
    }
}

#[maybe_async_cfg::maybe(
    sync(feature = "blocking", keep_self),
    async(not(feature = "blocking"), keep_self)
)]
impl<D: DelayNs> EpdParallelBus for LcdBus<D> {
    type Error = LcdBusError;

    async fn set_power(&mut self, on: bool) -> Result<(), Self::Error> {
        if on {
            self.oe.set_high();
            self.delay.delay_us(100).await;
            self.pwr.set_high();
            self.delay.delay_us(100).await;
            self.spv.set_high();
            self.delay.delay_ms(1).await;
        } else {
            self.delay.delay_ms(1).await;
            self.pwr.set_low();
            self.delay.delay_us(10).await;
            self.oe.set_low();
            self.delay.delay_us(100).await;
            self.spv.set_low();
        }
        Ok(())
    }

    async fn start_frame(&mut self) -> Result<(), Self::Error> {
        self.spv.set_low();
        self.delay.delay_us(1).await;
        self.ckv.set_low();
        self.delay.delay_us(3).await;
        self.ckv.set_high();
        self.delay.delay_us(1).await;
        self.spv.set_high();
        for _ in 0..3 {
            self.delay.delay_us(3).await;
            self.ckv.set_low();
            self.delay.delay_us(3).await;
            self.ckv.set_high();
        }
        Ok(())
    }

    async fn write_scanline(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        let (Some(i8080), Some(mut buf)) = (self.i8080.take(), self.buf.take()) else {
            return Err(LcdBusError::Lost);
        };
        if data.len() > buf.capacity() {
            self.i8080 = Some(i8080);
            self.buf = Some(buf);
            return Err(LcdBusError::TooLong);
        }
        buf.fill(data);
        self.le.set_low();
        match i8080.send(Command::<u8>::None, 0, buf) {
            Ok(transfer) => {
                // CKV rises once the DMA runs, so it is high for the data phase only and
                // not for the 6.5 us of `send()` setup before it.
                self.ckv.set_high();
                #[cfg(feature = "blocking")]
                let (result, i8080, buf) = transfer.wait();
                #[cfg(not(feature = "blocking"))]
                let (result, i8080, buf) = {
                    let mut transfer = transfer;
                    transfer.wait_for_done().await;
                    transfer.wait()
                };
                self.i8080 = Some(i8080);
                self.buf = Some(buf);
                result.map_err(LcdBusError::Dma)
            }
            Err((err, i8080, buf)) => {
                self.i8080 = Some(i8080);
                self.buf = Some(buf);
                Err(LcdBusError::Dma(err))
            }
        }
    }

    async fn next_row(&mut self) -> Result<(), Self::Error> {
        self.ckv.set_low();
        self.le.set_high();
        Ok(())
    }

    async fn end_frame(&mut self) -> Result<(), Self::Error> {
        self.le.set_low();
        self.ckv.set_high();
        Ok(())
    }
}
