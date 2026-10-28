//! Errors raised before any bytes reach the bus.

/// A caller-supplied buffer that cannot be honoured against the panel's declared dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
pub enum ValidationError {
    /// A buffer is smaller than the panel requires.
    BufferTooSmall {
        /// Minimum buffer length in bytes.
        required: usize,
        /// Actual length of the buffer the caller supplied.
        provided: usize,
    },
}
