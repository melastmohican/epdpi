# Changelog

## [Unreleased]

### Changed
- Now a workspace member. The register-level `fast_bus` moved to the new `epdpi-esp` crate and
  the `esp32s3` feature is gone from this crate. `GenericGpioBus::control_mut` is new, for
  wrapper buses.
- Edition 2024 and MSRV 1.85 (was edition 2021 and 1.75, copied from `epdsi` without
  revisiting). No source changes were needed beyond `rustfmt`'s 2024 import ordering.

### Added
- Initial driver: `EpdPanel`, `EpdParallelBus`, `GenericGpioBus`, `WaveformEngine`,
  `EpdFrameBuffer`, `EpdDisplay` and `EpdDisplayBuilder`, blocking and async.
- `Ed047tc1` panel (960 x 540, 16 grays) with the M5/FastEPD waveform matrix.
- `RefreshMode::{Clear, Quality16, FastMono}`.
- `RefreshMode::M5Quality`, M5GFX's `epd_quality` waveform (2 erase passes, 15 drawing passes),
  tables generated from `Panel_EPD.inl`. Needs a row data phase near 16 us.

### Fixed
- `GenericGpioBus` now follows M5GFX's `Bus_EPD` line sequence: LE stays high between rows
  and drops before the next row's data, the frame start uses M5GFX timing, and the bus waits
  1 ms after power-on. The earlier FastEPD-style sequence left noise on a PaperDIY panel.
  Confirmed on hardware with `RefreshMode::Clear`.
