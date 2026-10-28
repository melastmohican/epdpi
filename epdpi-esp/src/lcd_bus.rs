//! ESP32-S3 `LCD_CAM` i8080 bus with DMA, the way M5GFX drives the panel.
//!
//! The data pins D0..D7, the source clock CL (the i8080 write strobe) and SPH (the chip
//! select, low while a scanline is clocked) belong to the LCD peripheral, so the caller
//! configures an [`I8080`] with them. CKV, LE, SPV, OE and PWR stay plain GPIO, with CKV held
//! high for exactly the length of the DMA transfer, as M5GFX's `Bus_EPD` does. The row data
//! phase is then set by the bus clock: 22 us for a 248 byte row at 16 MHz on a PaperDIY,
//! against 62 us for the register-level GPIO bus.
//!
//! This bus is blocking. Needs the `esp32s3` and `blocking` features.

use esp_hal::Blocking;
use esp_hal::delay::Delay;
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

/// See the module docs.
pub struct LcdBus {
    i8080: Option<I8080<'static, Blocking>>,
    buf: Option<DmaTxBuf>,
    ckv: Output<'static>,
    spv: Output<'static>,
    oe: Output<'static>,
    le: Output<'static>,
    pwr: Output<'static>,
    delay: Delay,
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

impl LcdBus {
    /// Wraps a configured `I8080` (data pins, write strobe and chip select attached), a DMA
    /// buffer at least one scanline long (`EpdPanel::SCANLINE_BYTES`) and the control pins.
    pub fn new(
        i8080: I8080<'static, Blocking>,
        buf: DmaTxBuf,
        control: LcdControlPins<'static>,
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
            delay: Delay::new(),
        }
    }
}

impl EpdParallelBus for LcdBus {
    type Error = LcdBusError;

    fn set_power(&mut self, on: bool) -> Result<(), Self::Error> {
        if on {
            self.oe.set_high();
            self.delay.delay_micros(100);
            self.pwr.set_high();
            self.delay.delay_micros(100);
            self.spv.set_high();
            self.delay.delay_millis(1);
        } else {
            self.delay.delay_millis(1);
            self.pwr.set_low();
            self.delay.delay_micros(10);
            self.oe.set_low();
            self.delay.delay_micros(100);
            self.spv.set_low();
        }
        Ok(())
    }

    fn start_frame(&mut self) -> Result<(), Self::Error> {
        self.spv.set_low();
        self.delay.delay_micros(1);
        self.ckv.set_low();
        self.delay.delay_micros(3);
        self.ckv.set_high();
        self.delay.delay_micros(1);
        self.spv.set_high();
        for _ in 0..3 {
            self.delay.delay_micros(3);
            self.ckv.set_low();
            self.delay.delay_micros(3);
            self.ckv.set_high();
        }
        Ok(())
    }

    fn write_scanline(&mut self, data: &[u8]) -> Result<(), Self::Error> {
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
        self.ckv.set_high();
        match i8080.send(Command::<u8>::None, 0, buf) {
            Ok(transfer) => {
                let (result, i8080, buf) = transfer.wait();
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

    fn next_row(&mut self) -> Result<(), Self::Error> {
        self.ckv.set_low();
        self.le.set_high();
        Ok(())
    }

    fn end_frame(&mut self) -> Result<(), Self::Error> {
        self.le.set_low();
        self.ckv.set_high();
        Ok(())
    }
}
