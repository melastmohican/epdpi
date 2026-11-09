//! # `epdpi-esp`: ESP32 buses for `epdpi`
//!
//! Faster [`EpdParallelBus`](epdpi::EpdParallelBus) implementations for the `epdpi` e-paper
//! driver, kept out of the core crate so that crate never depends on a vendor HAL.
//!
//! The 16-gray waveforms are built for M5GFX's row data phase of 15.5 us (computed: 248 bytes
//! at 16 MHz). Bit-banging through `epdpi::GenericGpioBus` takes about 250 us, and even register
//! writes take about 62 us on an ESP32-S3, which squashes the gray ramp. These buses get closer:
//!
//! | Bus | Row data phase (248 bytes) | Needs |
//! | :--- | :--- | :--- |
//! | [`fast_bus::Esp32s3Bus`] | about 62 us | any ESP32-S3 HAL, no feature |
//! | `lcd_bus::LcdBus` | about 22 us measured | `esp32s3` feature, `esp-hal`. Blocking and async |
//! | `shift_reg_bus::ShiftRegBus` | set by the row time, 22 us by default | `esp32s3` feature, `esp-hal`. Blocking and async |
//!
//! The 6.5 us that `LcdBus` takes over the computed 15.5 us is the `send()` setup, which was
//! not isolated. A whole `LcdBus` row, data phase and edges, takes about 117 us (computed,
//! not measured here), against about 91 us for M5GFX.
//!
//! # Wiring
//!
//! | Wiring of the control lines | Bus |
//! | :--- | :--- |
//! | CKV, LE, SPV, OE and PWR on plain GPIO (M5Stack PaperDIY) | `lcd_bus::LcdBus` |
//! | STV, latch, OE, mode and the panel rails behind a 74HC595 style shift register (LilyGo T5 4.7 S3 V2.4) | `shift_reg_bus::ShiftRegBus` |
//! | Anything else | your own [`EpdParallelBus`](epdpi::EpdParallelBus) |
//!
//! A bus fits one wiring: the edge sequence, the order of the panel rails and the row timing
//! differ between boards. `ShiftRegBus` uses [`shift_reg::ShiftReg`], which also runs on the
//! host.
//!
//! # Chips
//!
//! Only the ESP32-S3 exists and is tested, on an M5Stack PaperDIY and a LilyGo T5 4.7 S3 V2.4. The chip is a Cargo feature,
//! as in `esp-hal`, so other chips can be added later. A classic ESP32 or an S2 would need an
//! I2S parallel bus, which is not written.
//!
//! # `esp-hal` version
//!
//! `esp-hal` types appear in the public API (`Output`, `I8080`, the DMA types), so this crate
//! follows `esp-hal` closely. `epdpi-esp` 0.1 works with `esp-hal` 1.1.x (`~1.1.0`, with its
//! `unstable` feature). A new major or minor version of `esp-hal` needs a new `epdpi-esp`.
//!
//! # Cargo features
//!
//! - `blocking` (default): blocking API, forwarded to `epdpi`.
//! - `esp32s3`: enables `esp-hal`, `lcd_bus` and `shift_reg_bus`.
//!
//! ```toml
//! epdpi = "0.2"
//! epdpi-esp = { version = "0.2", features = ["esp32s3"] }
//! ```
#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod fast_bus;
pub mod shift_reg;

#[cfg(feature = "esp32s3")]
#[cfg_attr(docsrs, doc(cfg(feature = "esp32s3")))]
pub mod lcd_bus;

#[cfg(feature = "esp32s3")]
#[cfg_attr(docsrs, doc(cfg(feature = "esp32s3")))]
pub mod shift_reg_bus;

/// Prose-only stand-in for docs.rs, which cannot build the `esp32s3` feature. The real module
/// is above, with the same text.
#[cfg(all(docsrs, not(feature = "esp32s3")))]
#[doc = include_str!("../docs/lcd_bus.md")]
pub mod lcd_bus {}

/// Prose-only stand-in for docs.rs, as for `lcd_bus`.
#[cfg(all(docsrs, not(feature = "esp32s3")))]
#[doc = include_str!("../docs/shift_reg_bus.md")]
pub mod shift_reg_bus {}
