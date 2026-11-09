//! Bit-banged writer for a 74HC595 style shift register (the 74HCT4094 on the LilyGo T5 4.7 S3
//! V2.4 speaks the same protocol).
//!
//! Three output pins drive it: data, clock and store. The last byte written is cached, so a
//! single bit can change with [`ShiftReg::set`] and [`ShiftReg::clear`]. The pins are any
//! `embedded-hal` outputs, so the sequence runs on the host in the tests.

use embedded_hal::digital::OutputPin;

/// Burns a few dozen CPU cycles so the register sees its setup and pulse widths. Back to back
/// GPIO writes at full clock speed are too close together for a 74HC595 at 3.3 V.
#[inline(always)]
fn settle() {
    for _ in 0..16 {
        core::hint::spin_loop();
    }
}

/// A shift register on three output pins, with the last value written cached.
#[derive(Debug)]
pub struct ShiftReg<P> {
    data: P,
    clock: P,
    store: P,
    state: u8,
}

impl<P: OutputPin> ShiftReg<P> {
    /// Takes the three pins and writes `state` out at once, so the outputs start in a known
    /// state.
    pub fn new(data: P, clock: P, store: P, state: u8) -> Result<Self, P::Error> {
        let mut reg = Self {
            data,
            clock,
            store,
            state,
        };
        reg.write()?;
        Ok(reg)
    }

    /// The byte last written.
    pub fn state(&self) -> u8 {
        self.state
    }

    /// Shifts the cached byte out, most significant bit first, and latches it.
    fn write(&mut self) -> Result<(), P::Error> {
        self.store.set_low()?;
        let mut byte = self.state;
        for _ in 0..8 {
            self.clock.set_low()?;
            if byte & 0x80 != 0 {
                self.data.set_high()?;
            } else {
                self.data.set_low()?;
            }
            byte <<= 1;
            settle();
            self.clock.set_high()?;
            settle();
        }
        self.store.set_high()?;
        settle();
        Ok(())
    }

    /// Sets the bits in `mask` and writes.
    pub fn set(&mut self, mask: u8) -> Result<(), P::Error> {
        self.state |= mask;
        self.write()
    }

    /// Clears the bits in `mask` and writes.
    pub fn clear(&mut self, mask: u8) -> Result<(), P::Error> {
        self.state &= !mask;
        self.write()
    }
}
