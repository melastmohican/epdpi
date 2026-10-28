//! Prelude re-exporting the common traits, drivers and helpers.

pub use crate::bus::{ControlPins, GenericGpioBus};
pub use crate::driver::{EpdDisplay, EpdDisplayBuilder};
pub use crate::error::ValidationError;
pub use crate::framebuffer::EpdFrameBuffer;
pub use crate::panels::{Ed047tc1, ED047TC1_LUT};
pub use crate::traits::{ColorMode, DriveCode, EpdPanel, EpdParallelBus};
pub use crate::waveform::{RefreshMode, WaveformEngine};
