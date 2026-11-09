#![doc = include_str!("../docs/shift_reg_bus.md")]

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
use esp_hal::time::Instant;

use epdpi::EpdParallelBus;

use crate::shift_reg::ShiftReg;

/// The bits of the shift register byte, as wired on the V2.4 (schematic outputs QP0 to QP7).
pub mod bits {
    /// Latch enable (EP_LE): clocks the source drivers' row into the outputs.
    pub const LATCH: u8 = 0x01;
    /// Active high: holds the panel supplies off.
    pub const POWER_DISABLE: u8 = 0x02;
    /// Positive rail enable.
    pub const POS_POWER: u8 = 0x04;
    /// Negative rail enable.
    pub const NEG_POWER: u8 = 0x08;
    /// Gate start pulse (SPV), active low during the frame start.
    pub const STV: u8 = 0x10;
    /// Scan direction.
    pub const SCAN_DIR: u8 = 0x20;
    /// Mode: high while a frame is being driven.
    pub const MODE: u8 = 0x40;
    /// Source output enable.
    pub const OUTPUT_ENABLE: u8 = 0x80;

    /// The value the register starts with: supplies disabled, scan direction set, STV high.
    pub const INITIAL: u8 = POWER_DISABLE | SCAN_DIR | STV;
}

use bits::{
    INITIAL, LATCH, MODE, NEG_POWER, OUTPUT_ENABLE, POS_POWER, POWER_DISABLE, SCAN_DIR, STV,
};

/// Why a bus call failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ShiftRegBusError {
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

/// Time CKV stays high per row, in microseconds. The vendor library uses 50 us at 10 MHz. 22 us
/// at 16 MHz is the PaperDIY timing the `epdpi` waveforms were tuned for, and gave better grays
/// on the V2.4.
pub const ROW_US: u64 = 22;

/// See the module docs. `D` times the settle delays and the frame start pulses.
pub struct ShiftRegBus<D> {
    i8080: Option<I8080<'static, Mode>>,
    buf: Option<DmaTxBuf>,
    ckv: Output<'static>,
    reg: ShiftReg<Output<'static>>,
    sph: Output<'static>,
    row_us: u64,
    delay: D,
}

/// Builds the shift register on the V2.4's data, clock and store pins, started with
/// [`bits::INITIAL`].
pub fn register(
    data: Output<'static>,
    clock: Output<'static>,
    store: Output<'static>,
) -> ShiftReg<Output<'static>> {
    let Ok(reg) = ShiftReg::new(data, clock, store, INITIAL);
    reg
}

impl<D> ShiftRegBus<D> {
    /// Wraps a configured `I8080` (data pins and write strobe attached, no chip select), a DMA
    /// buffer at least one scanline long (`EpdPanel::SCANLINE_BYTES`), CKV, the shift register
    /// (from [`register`]), SPH and a delay.
    ///
    /// SPH is not the LCD chip select on this board: the bus drives it by hand, low only while a
    /// row is sent. Pass it high.
    pub fn new(
        i8080: I8080<'static, Mode>,
        buf: DmaTxBuf,
        ckv: Output<'static>,
        reg: ShiftReg<Output<'static>>,
        sph: Output<'static>,
        delay: D,
    ) -> Self {
        Self {
            i8080: Some(i8080),
            buf: Some(buf),
            ckv,
            reg,
            sph,
            row_us: ROW_US,
            delay,
        }
    }

    /// Sets how long CKV stays high per row, in microseconds. The default is [`ROW_US`]. A longer
    /// row drives the pixels harder per pass: use about 50 for repair and clear cycles, as the
    /// vendor library does.
    pub fn set_row_us(&mut self, us: u64) {
        self.row_us = us;
    }

    fn reg_set(&mut self, mask: u8) {
        let Ok(()) = self.reg.set(mask);
    }

    fn reg_clear(&mut self, mask: u8) {
        let Ok(()) = self.reg.clear(mask);
    }
}

#[maybe_async_cfg::maybe(
    sync(feature = "blocking", keep_self),
    async(not(feature = "blocking"), keep_self)
)]
impl<D: DelayNs> ShiftRegBus<D> {
    /// CKV high for `high` microseconds, then low for `low`.
    async fn pulse_ckv(&mut self, high: u32, low: u32) {
        self.ckv.set_high();
        self.delay.delay_us(high).await;
        self.ckv.set_low();
        self.delay.delay_us(low).await;
    }
}

#[maybe_async_cfg::maybe(
    sync(feature = "blocking", keep_self),
    async(not(feature = "blocking"), keep_self)
)]
impl<D: DelayNs> EpdParallelBus for ShiftRegBus<D> {
    type Error = ShiftRegBusError;

    /// Vendor `epd_poweron` and `epd_poweroff`.
    async fn set_power(&mut self, on: bool) -> Result<(), Self::Error> {
        if on {
            self.reg_set(SCAN_DIR);
            self.reg_clear(POWER_DISABLE);
            self.delay.delay_ms(1).await;
            self.reg_set(NEG_POWER);
            self.delay.delay_ms(1).await;
            self.reg_set(POS_POWER);
            self.delay.delay_ms(1).await;
            self.reg_set(STV);
            self.delay.delay_ms(1).await;
        } else {
            self.reg_clear(POS_POWER);
            self.reg_clear(NEG_POWER);
            self.reg_set(POWER_DISABLE);
            self.reg_clear(STV);
        }
        Ok(())
    }

    /// Vendor `epd_start_frame`.
    async fn start_frame(&mut self) -> Result<(), Self::Error> {
        self.reg_set(MODE);
        self.pulse_ckv(1, 1).await;
        self.reg_clear(STV);
        self.delay.delay_us(1).await;
        self.ckv.set_high();
        self.reg_set(STV);
        self.delay.delay_us(10).await;
        self.ckv.set_low();
        self.delay.delay_us(10).await;
        self.pulse_ckv(10, 0).await;
        self.reg_set(OUTPUT_ENABLE);
        self.pulse_ckv(1, 1).await;
        Ok(())
    }

    /// Clocks one row out while CKV is high for the row time, as the vendor's `epd_output_row`.
    async fn write_scanline(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        let (Some(i8080), Some(mut buf)) = (self.i8080.take(), self.buf.take()) else {
            return Err(ShiftRegBusError::Lost);
        };
        if data.len() > buf.capacity() {
            self.i8080 = Some(i8080);
            self.buf = Some(buf);
            return Err(ShiftRegBusError::TooLong);
        }
        buf.fill(data);
        self.sph.set_low();
        let high = Instant::now();
        match i8080.send(Command::<u8>::None, 0, buf) {
            Ok(transfer) => {
                self.ckv.set_high();
                #[cfg(feature = "blocking")]
                let (result, i8080, buf) = transfer.wait();
                #[cfg(not(feature = "blocking"))]
                let (result, i8080, buf) = {
                    let mut transfer = transfer;
                    transfer.wait_for_done().await;
                    transfer.wait()
                };
                self.sph.set_high();
                self.i8080 = Some(i8080);
                self.buf = Some(buf);
                while (Instant::now() - high).as_micros() < self.row_us {}
                self.ckv.set_low();
                self.delay.delay_us(5).await;
                result.map_err(ShiftRegBusError::Dma)
            }
            Err((err, i8080, buf)) => {
                self.i8080 = Some(i8080);
                self.buf = Some(buf);
                Err(ShiftRegBusError::Dma(err))
            }
        }
    }

    async fn next_row(&mut self) -> Result<(), Self::Error> {
        self.reg_set(LATCH);
        self.reg_clear(LATCH);
        Ok(())
    }

    /// Vendor `epd_end_frame`.
    async fn end_frame(&mut self) -> Result<(), Self::Error> {
        self.reg_clear(OUTPUT_ENABLE);
        self.reg_clear(MODE);
        self.pulse_ckv(1, 1).await;
        self.pulse_ckv(1, 1).await;
        Ok(())
    }
}
