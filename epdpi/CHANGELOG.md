# Changelog

## [Unreleased]

## [0.3.0] - 2026-11-11

### Added
- `dither` turns 8 bit gray into levels the way M5GFX does, with its 4 by 4 Bayer matrix, for the 16 level modes and the black and white ones. It has `gray8_to_level`, `gray8_to_bw` and the `EpdFrameBuffer` setters `set_logical_pixel_gray8` and `set_logical_pixel_bw8`. A picture drawn with them matches M5GFX's for the same 8 bit input; `Gray4` input is still taken as it is.

### Notes
- The crate description, the README and the crate docs name the LilyGo T5 4.7 S3 V2.4 next to the M5Stack PaperDIY. The `FastMono` doc has a page-turn recipe.
- There is no `sleep()`: `EpdDisplay` already switches the panel rails off after every refresh, also when it fails. The README (Buses) and `EpdDisplay::release` say what to do before a deep sleep.

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

[Unreleased]: https://github.com/melastmohican/epdpi/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/melastmohican/epdpi/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/melastmohican/epdpi/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/melastmohican/epdpi/releases/tag/v0.1.0
