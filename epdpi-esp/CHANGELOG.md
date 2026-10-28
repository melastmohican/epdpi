# Changelog

## [Unreleased]

### Added
- `fast_bus`: register-level `FastGpioBus` and `Esp32s3Bus`, moved from `epdpi`.
- `lcd_bus` (feature `esp32s3`, blocking only): `LcdBus` over `LCD_CAM` i8080 with DMA, 22 us
  per 248 byte row on a PaperDIY.
