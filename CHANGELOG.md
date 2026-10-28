# Changelog

## [Unreleased]

### Added
- Initial driver: `EpdPanel`, `EpdParallelBus`, `GenericGpioBus`, `WaveformEngine`,
  `EpdFrameBuffer`, `EpdDisplay` and `EpdDisplayBuilder`, blocking and async.
- `Ed047tc1` panel (960 x 540, 16 grays) with the M5/FastEPD waveform matrix.
- `RefreshMode::{Clear, Quality16, FastMono}`.

### Fixed
- `GenericGpioBus` now follows M5GFX's `Bus_EPD` line sequence: LE stays high between rows
  and drops before the next row's data, the frame start uses M5GFX timing, and the bus waits
  1 ms after power-on. The earlier FastEPD-style sequence left noise on a PaperDIY panel.
  Confirmed on hardware with `RefreshMode::Clear`.
