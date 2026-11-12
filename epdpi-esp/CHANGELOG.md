# Changelog

## [Unreleased]

## [0.3.0] - 2026-11-11

### Notes
- Requires `epdpi` 0.3.0, released together with it. No code change in this crate.
- `LcdBus` documents the extra CL cycle that the `LCD_CAM` hardware clocks in every row (one more than the row has bytes, measured on a PaperDIY), what it sees on the data lines and what it can do at the edges of the picture, see "The extra clock" in the `lcd_bus` docs. No code change.

## [0.2.0] - 2026-11-09

### Added
- `shift_reg_bus::ShiftRegBus`, a bus for the LilyGo T5 4.7 S3 V2.4, where a 74HC595 style shift register carries STV, latch, OE, mode and the panel rails. Blocking and async, behind the `esp32s3` feature. `shift_reg::ShiftReg` is the register writer, on any `embedded-hal` pins and tested on the host.

## [0.1.0] - 2026-10-31

First release.

### Added
- A fast `LCD_CAM` and DMA bus for the ESP32-S3. `lcd_bus::LcdBus` streams a 248 byte row in 22 us on an M5Stack PaperDIY, the row data phase the M5GFX style waveforms of `epdpi` need. It needs the `esp32s3` feature and `esp-hal`, in blocking or async mode.
- A register-level GPIO bus that needs no `esp-hal`. `fast_bus::FastGpioBus` and `Esp32s3Bus` write the ESP32-S3 GPIO registers directly, about 62 us per row: much faster than plain `embedded-hal` pins, but still too slow for an even 16 gray ramp.
- An optional `defmt` feature.

### Notes
- This release works with `esp-hal` 1.1.x only. `esp-hal` types are part of the public API (`Output`, `I8080`, the DMA types), so a new `esp-hal` version needs a new `epdpi-esp` version.
- `LcdBusError` and `PinMapError` are `#[non_exhaustive]`, so a `match` outside the crate needs a wildcard arm.
- The minimum supported Rust version is 1.88, from `esp-hal`.

[Unreleased]: https://github.com/melastmohican/epdpi/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/melastmohican/epdpi/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/melastmohican/epdpi/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/melastmohican/epdpi/releases/tag/v0.1.0
