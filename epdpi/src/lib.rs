//! # `epdpi`: E-Paper Display Parallel Interface framework
//!
//! A `no_std`, [`embedded-hal`] 1.0 compatible driver framework for raw-parallel
//! Electronic Paper Displays, where the MCU drives the panel's gate and source drivers
//! directly over an 8-bit data bus and runs the waveform itself. The first supported
//! glass is the E Ink ED047TC1 (960 x 540, 16 grays) on the M5Stack PaperDIY.
//! It is the parallel counterpart of `epdsi`, which covers SPI panels with their own
//! controller IC.
//!
//! # Architecture
//!
//! - [`EpdPanel`]: a zero-sized type with the glass geometry and its waveform matrix.
//! - [`EpdParallelBus`]: the transport, five operations that pulse the gate and source
//!   control lines and stream one scanline. [`GenericGpioBus`] implements it on any
//!   `embedded-hal` pins.
//! - [`WaveformEngine`]: stateless conversion of framebuffer rows to 2 bits per pixel
//!   drive codes. All panel numbers come from [`EpdPanel`].
//! - [`EpdFrameBuffer`]: 4 bits per pixel buffer in caller memory, an `embedded-graphics`
//!   `DrawTarget` over `Gray4` with the `graphics` feature.
//! - [`EpdDisplay`]: power sequencing and refresh, built with [`EpdDisplayBuilder`].
//!
//! # Cargo features
//!
//! - `blocking` (default): `embedded-hal` 1.0 API. Turn off with `default-features = false`
//!   to get the `embedded-hal-async` API, where delays and bus calls are `async`.
//! - `graphics` (default): `embedded-graphics-core` support for [`EpdFrameBuffer`].
//! - `defmt`: `defmt::Format` on the public types.
//!
//! Faster buses for specific chips live in separate crates, for example `epdpi-esp`, so this
//! crate never depends on a vendor HAL. The 16-gray waveforms assume a row data phase near
//! 16 us, which bit-banging through [`GenericGpioBus`] cannot reach.
//!
//! [`embedded-hal`]: https://docs.rs/embedded-hal

#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod bus;
pub mod driver;
pub mod error;
pub mod framebuffer;
pub mod panels;
pub mod prelude;
pub mod traits;
pub mod waveform;

pub use crate::bus::{ControlPins, GenericGpioBus};
pub use crate::driver::{EpdDisplay, EpdDisplayBuilder};
pub use crate::error::ValidationError;
pub use crate::framebuffer::EpdFrameBuffer;
pub use crate::panels::{
    ED047TC1_LUT, Ed047tc1, M5_ERASER_LUT, M5_FAST_LUT, M5_FASTEST_LUT, M5_QUALITY_LUT, M5_TEXT_LUT,
};
pub use crate::traits::{ColorMode, DriveCode, EpdPanel, EpdParallelBus};
pub use crate::waveform::{RefreshMode, WaveformEngine};
