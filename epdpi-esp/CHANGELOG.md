# Changelog

## [Unreleased]

### Changed
- `LcdBusError` and `PinMapError` are `#[non_exhaustive]`: a variant can be added in a minor version, so
  a `match` outside the crate needs a wildcard arm.
- The README and the crate docs say that `esp-hal` types are part of the public API and that 0.1 works
  with `esp-hal` 1.1.x only.

### Added
- `fast_bus`: register-level `FastGpioBus` and `Esp32s3Bus`, moved from `epdpi`.
- `lcd_bus` (feature `esp32s3`, blocking only): `LcdBus` over `LCD_CAM` i8080 with DMA, 22 us
  per 248 byte row on a PaperDIY.
