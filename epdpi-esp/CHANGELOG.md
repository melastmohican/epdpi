# Changelog

## [Unreleased]

## [0.1.0] - 2026-10-31

First release.

### Added
- `lcd_bus::LcdBus`, a bus for the ESP32-S3 that streams rows through the `LCD_CAM` peripheral with DMA, 22 us per 248 byte row on an M5Stack PaperDIY. It is the bus that reaches the row data phase the M5GFX style waveforms of `epdpi` need. It needs the `esp32s3` feature and `esp-hal`, in blocking or async mode.
- `fast_bus::FastGpioBus` and `Esp32s3Bus`, a bus that writes the ESP32-S3 GPIO registers directly, about 62 us per row. It needs no feature and no `esp-hal`, and is much faster than plain `embedded-hal` pins but still too slow for an even 16 gray ramp.
- An optional `defmt` feature.

### Notes
- This release works with `esp-hal` 1.1.x only. `esp-hal` types are part of the public API (`Output`, `I8080`, the DMA types), so a new `esp-hal` version needs a new `epdpi-esp` version.
- `LcdBusError` and `PinMapError` are `#[non_exhaustive]`, so a `match` outside the crate needs a wildcard arm.
- The minimum supported Rust version is 1.88, from `esp-hal`.

[Unreleased]: https://github.com/melastmohican/epdpi/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/melastmohican/epdpi/releases/tag/v0.1.0
