# Changelog

## [Unreleased]

### Added
- `dither`, M5GFX's 4 by 4 Bayer reduction of 8 bit gray: `gray8_to_level` for the 16 level modes and `gray8_to_bw` for `M5Fast` and `M5Fastest`, with `EpdFrameBuffer::set_logical_pixel_gray8` and `set_logical_pixel_bw8`. A picture drawn with them matches M5GFX's for the same 8 bit input; `Gray4` input is still taken as it is.

## [0.2.0] - 2026-11-09

### Changed
- Breaking: `Rotation` and `DriveCode` are `#[non_exhaustive]`, so a `match` outside the crate needs a wildcard arm and a variant added later does not break it (the same as `RefreshMode` and `ColorMode` since 0.1.0).

## [0.1.0] - 2026-10-31

First release.

### Added
- A `no_std` driver for raw-parallel e-paper panels, blocking or async. The microcontroller drives the gate and source drivers itself, through `embedded-hal` 1.0, or `embedded-hal-async` with `default-features = false`.
- The `Ed047tc1` panel (960 x 540, 16 grays), tested on an M5Stack PaperDIY.
- Seven refresh modes, four of them following M5GFX. `RefreshMode` has `Clear`, `Quality16` and `FastMono`, and `M5Quality`, `M5Text`, `M5Fast` and `M5Fastest` with waveform tables generated from M5GFX `Panel_EPD`. The M5GFX modes need a row data phase near 16 us, which the buses of `epdpi-esp` reach.
- A 4 bits per pixel frame buffer in your own memory. `EpdFrameBuffer` is an `embedded-graphics` draw target over `Gray4` with the `graphics` feature, and it turns in 90 degree steps with `Rotation`.
- Window updates, so only the part of the panel that changed is refreshed. `Rect`, `EpdFrameBuffer::mark_dirty`, `EpdDisplay::refresh_rect` and `refresh_dirty` do it.
- A portable GPIO bus on any `embedded-hal` output pins. `GenericGpioBus` rows take about 250 us, so the 16 gray waveforms squash into a few tones: use `epdpi-esp` on an ESP32-S3.
- An optional `defmt` feature.

### Notes
- `RefreshMode` and `ColorMode` are `#[non_exhaustive]`, so a `match` outside the crate needs a wildcard arm and a mode added later does not break it.
- `WaveformEngine` and `M5Pass` stay public under `epdpi::waveform` but are hidden from the documentation and not covered by semver.
- A method added to `EpdParallelBus` later gets a default body, so a bus written outside this workspace keeps compiling.
- The minimum supported Rust version is 1.85.

[Unreleased]: https://github.com/melastmohican/epdpi/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/melastmohican/epdpi/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/melastmohican/epdpi/releases/tag/v0.1.0
