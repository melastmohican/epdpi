#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! `ShiftReg` against simulated pins: the bit order, the store pulse and the cached byte.

use core::convert::Infallible;
use std::cell::RefCell;
use std::rc::Rc;

use embedded_hal::digital::{ErrorType, OutputPin};
use epdpi_esp::shift_reg::ShiftReg;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Line {
    Data,
    Clock,
    Store,
}

type Log = Rc<RefCell<Vec<(Line, bool)>>>;

struct Pin {
    line: Line,
    log: Log,
}

impl ErrorType for Pin {
    type Error = Infallible;
}

impl OutputPin for Pin {
    fn set_low(&mut self) -> Result<(), Infallible> {
        self.log.borrow_mut().push((self.line, false));
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Infallible> {
        self.log.borrow_mut().push((self.line, true));
        Ok(())
    }
}

fn reg(state: u8) -> (ShiftReg<Pin>, Log) {
    let log: Log = Rc::default();
    let pin = |line| Pin {
        line,
        log: Rc::clone(&log),
    };
    let reg = ShiftReg::new(pin(Line::Data), pin(Line::Clock), pin(Line::Store), state).unwrap();
    (reg, log)
}

/// The bytes latched so far: the data line sampled at each clock rising edge, and the byte
/// taken when the store line rises. Also checks the clock pulses 8 times per byte and that the
/// store is low for all of them.
fn latched(log: &Log) -> Vec<u8> {
    let mut data = false;
    let mut clock = false;
    let mut store = true;
    let mut bits: Vec<bool> = Vec::new();
    let mut out = Vec::new();
    for &(line, level) in log.borrow().iter() {
        match line {
            Line::Data => data = level,
            Line::Clock => {
                if level && !clock {
                    assert!(!store, "clocked while the store line was high");
                    bits.push(data);
                }
                clock = level;
            }
            Line::Store => {
                if level && !store {
                    assert_eq!(bits.len(), 8, "store rose after {} clocks", bits.len());
                    out.push(
                        bits.iter()
                            .fold(0u8, |byte, &bit| (byte << 1) | u8::from(bit)),
                    );
                    bits.clear();
                }
                store = level;
            }
        }
    }
    assert!(bits.is_empty(), "bits shifted and never latched");
    out
}

#[test]
fn new_writes_the_initial_state() {
    let (reg, log) = reg(0b0011_0010);
    assert_eq!(reg.state(), 0b0011_0010);
    assert_eq!(latched(&log), [0b0011_0010]);
}

#[test]
fn bits_go_out_most_significant_first() {
    let (_reg, log) = reg(0b1000_0010);
    let data: Vec<bool> = {
        let mut clock = false;
        let mut data = false;
        let mut bits = Vec::new();
        for &(line, level) in log.borrow().iter() {
            match line {
                Line::Data => data = level,
                Line::Clock => {
                    if level && !clock {
                        bits.push(data);
                    }
                    clock = level;
                }
                Line::Store => {}
            }
        }
        bits
    };
    assert_eq!(data, [true, false, false, false, false, false, true, false]);
}

#[test]
fn set_and_clear_change_only_their_bits() {
    let (mut reg, log) = reg(0x02);
    reg.set(0x14).unwrap();
    assert_eq!(reg.state(), 0x16);
    reg.clear(0x02).unwrap();
    assert_eq!(reg.state(), 0x14);
    reg.clear(0x80).unwrap();
    assert_eq!(reg.state(), 0x14);
    reg.set(0x14).unwrap();
    assert_eq!(reg.state(), 0x14);
    assert_eq!(latched(&log), [0x02, 0x16, 0x14, 0x14, 0x14]);
}

#[test]
fn store_is_low_while_shifting_and_high_after() {
    let (_reg, log) = reg(0xA5);
    let log = log.borrow();
    assert_eq!(log.first(), Some(&(Line::Store, false)));
    assert_eq!(log.last(), Some(&(Line::Store, true)));
    assert_eq!(log.iter().filter(|e| **e == (Line::Store, true)).count(), 1);
}
