# Changelog

## [Unreleased]

## [0.1.0] - 2026-10-31

First release.

### Added
- A `no_std` driver for raw-parallel e-paper panels, where the microcontroller drives the gate and source drivers itself. It uses `embedded-hal` 1.0 and is blocking by default, or async with `default-features = false`.
- The `Ed047tc1` panel (960 x 540, 16 grays), tested on an M5Stack PaperDIY.
- Seven refresh modes in `RefreshMode`: `Clear`, `Quality16` and `FastMono`, and four that follow M5GFX, `M5Quality`, `M5Text`, `M5Fast` and `M5Fastest`, with waveform tables generated from M5GFX `Panel_EPD`. The M5GFX modes need a row data phase near 16 us, which the buses of `epdpi-esp` reach.
- `EpdFrameBuffer`, a 4 bits per pixel buffer in your memory. With the `graphics` feature it is an `embedded-graphics` draw target over `Gray4`, and it turns in 90 degree steps with `Rotation`.
- Window updates: `Rect`, `EpdFrameBuffer::mark_dirty`, `EpdDisplay::refresh_rect` and `refresh_dirty` refresh only the part of the panel that changed.
- `GenericGpioBus`, a portable bus on any `embedded-hal` output pins. Its rows take about 250 us, so the 16 gray waveforms squash into a few tones: use `epdpi-esp` on an ESP32-S3.
- An optional `defmt` feature.

### Notes
- `RefreshMode` and `ColorMode` are `#[non_exhaustive]`, so a `match` outside the crate needs a wildcard arm and a mode added later does not break it.
- `WaveformEngine` and `M5Pass` stay public under `epdpi::waveform` but are hidden from the documentation and not covered by semver.
- A method added to `EpdParallelBus` later gets a default body, so a bus written outside this workspace keeps compiling.
- The minimum supported Rust version is 1.85.

[Unreleased]: https://github.com/melastmohican/epdpi/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/melastmohican/epdpi/releases/tag/v0.1.0
