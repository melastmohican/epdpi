ESP32-S3 `LCD_CAM` i8080 bus with DMA, the way M5GFX drives the panel.

The data pins D0..D7, the source clock CL (the i8080 write strobe) and SPH (the chip
select, low while a scanline is clocked) belong to the LCD peripheral, so the caller
configures an `I8080` with them. CKV, LE, SPV, OE and PWR stay plain GPIO, with CKV held
high for exactly the length of the DMA transfer, as M5GFX's `Bus_EPD` does. The row data
phase is then set by the bus clock: 22 us for a 248 byte row at 16 MHz on a PaperDIY,
against 62 us for the register-level GPIO bus.

Works in both modes. With the `blocking` feature (default) the I8080 is `Blocking` and the
delay is any `embedded_hal::delay::DelayNs`, such as `esp_hal::delay::Delay`. Without it
the I8080 is `Async`, a row waits for the DMA with `wait_for_done`, and the delay is an
`embedded_hal_async::delay::DelayNs`, such as `embassy_time::Delay`. Needs the `esp32s3`
feature.

# Building these docs

docs.rs cannot build this module: `esp-hal` with `esp32s3` needs the Xtensa target, which docs.rs
does not have. This page is the prose only. The full API pages (`LcdBus`, `LcdControlPins`,
`LcdBusError`) build locally with the `esp` toolchain:

```text
cargo +esp doc --no-deps --target xtensa-esp32s3-none-elf -Zbuild-std=core \
    -p epdpi-esp --features esp32s3 --open
```

# Example

```ignore
// `i8080` is an esp_hal I8080 with data pins, WRX (CL) and CS (SPH) attached, `buf` a DmaTxBuf of
// at least `EpdPanel::SCANLINE_BYTES`, and the five control pins are plain GPIO outputs.
let bus = LcdBus::new(
    i8080,
    buf,
    LcdControlPins { ckv, spv, oe, le, pwr },
    esp_hal::delay::Delay::new(), // or embassy_time::Delay without the `blocking` feature
);
```
