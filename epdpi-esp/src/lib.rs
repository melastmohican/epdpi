//! # `epdpi-esp`: ESP32 buses for `epdpi`
//!
//! Faster [`EpdParallelBus`](epdpi::EpdParallelBus) implementations for the `epdpi` e-paper
//! driver, kept out of the core crate so that crate never depends on a vendor HAL.
//!
//! The 16-gray waveforms assume a row data phase near 16 us. Bit-banging through
//! `epdpi::GenericGpioBus` takes about 250 us, and even register writes take about 62 us on an
//! ESP32-S3, which squashes the gray ramp. These buses get closer:
//!
//! | Bus | Row data phase (248 bytes) | Needs |
//! | :--- | :--- | :--- |
//! | [`fast_bus::Esp32s3Bus`] | about 62 us | any ESP32-S3 HAL, no feature |
//! | `lcd_bus::LcdBus` | about 22 us | `esp32s3` and `blocking` features, `esp-hal` |
//!
//! # Chips
//!
//! Only the ESP32-S3 exists and is tested, on an M5Stack PaperDIY. The chip is a Cargo feature,
//! as in `esp-hal`, so other chips can be added later. A classic ESP32 or an S2 would need an
//! I2S parallel bus, which is not written.
//!
//! # Cargo features
//!
//! - `blocking` (default): blocking API, forwarded to `epdpi`.
//! - `esp32s3`: enables `esp-hal` and `lcd_bus`.
//!
//! ```toml
//! epdpi = "0.1"
//! epdpi-esp = { version = "0.1", features = ["esp32s3"] }
//! ```
#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod fast_bus;

#[cfg(all(feature = "esp32s3", feature = "blocking"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "esp32s3", feature = "blocking"))))]
pub mod lcd_bus;
