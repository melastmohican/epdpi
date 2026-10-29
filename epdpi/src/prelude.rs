//! Prelude re-exporting the common traits, drivers and helpers.

pub use crate::bus::{ControlPins, GenericGpioBus};
pub use crate::driver::{EpdDisplay, EpdDisplayBuilder};
pub use crate::error::ValidationError;
pub use crate::framebuffer::EpdFrameBuffer;
pub use crate::panels::{
    ED047TC1_LUT, Ed047tc1, M5_ERASER_LUT, M5_FAST_LUT, M5_FASTEST_LUT, M5_QUALITY_LUT, M5_TEXT_LUT,
};
pub use crate::traits::{ColorMode, DriveCode, EpdPanel, EpdParallelBus};
pub use crate::waveform::{RefreshMode, WaveformEngine};
