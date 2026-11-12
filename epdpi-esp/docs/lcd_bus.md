ESP32-S3 `LCD_CAM` i8080 bus with DMA, the way M5GFX drives the panel.

The data pins D0..D7, the source clock CL (the i8080 write strobe) and SPH (the chip
select, low while a scanline is clocked) belong to the LCD peripheral, so the caller
configures an `I8080` with them. CKV, LE, SPV, OE and PWR stay plain GPIO. CKV rises once
`send()` has returned and the DMA runs, so it is high for the data phase only. M5GFX's
`Bus_EPD` raises it before the DMA starts; this was chosen after a sweep and is not the same.
The row data phase is set by the bus clock: 15.5 us computed for a 248 byte row at 16 MHz
(the number the M5GFX waveforms are built for), 22 us measured on a PaperDIY, against 62 us
for the register-level GPIO bus. The 6.5 us over the computed value is the `send()` setup,
which was not isolated. A whole row takes about 117 us (computed, not measured here), against
about 91 us for M5GFX.

`LcdBus` expects CKV, LE, SPV, OE and PWR on plain GPIO pins, as on the M5Stack PaperDIY. A
board that routes them through a 74HC595 style shift register, such as the LilyGo T5 4.7 S3
V2.4, needs [`ShiftRegBus`](crate::shift_reg_bus) instead, and any other wiring needs its own
`EpdParallelBus`.

Works in both modes. With the `blocking` feature (default) the I8080 is `Blocking` and the
delay is any `embedded_hal::delay::DelayNs`, such as `esp_hal::delay::Delay`. Without it
the I8080 is `Async`, a row waits for the DMA with `wait_for_done`, and the delay is an
`embedded_hal_async::delay::DelayNs`, such as `embassy_time::Delay`. Needs the `esp32s3`
feature.

# The extra clock

The `LCD_CAM` hardware clocks one more CL cycle per row than the row has bytes. On a PaperDIY every
one of 540 rows of 248 bytes showed 249 CL cycles, rising and falling edges alike, and every row of 256
bytes showed 257, all of them while SPH is low. The extra cycle happens while the data lines are at their
idle level: a row with D0 high in every byte still counted one cycle with D0 low, and D0 is low after the
transfer. FastEPD documents the same quirk of this hardware and says the extra clock comes before the
data. The idle level fits that, but a count cannot tell before from after, so before is likely and not
proven.

If it comes before the data, byte 0 of every row is the idle level (hold) and the row lands one byte,
four pixels, to the right of where the code puts it. Then x = 0 to 3 are not driven by the gray passes
and x = 956 to 959 would fall off the end of the 240 byte row. Stripes drawn at both edges of a PaperDIY
did not contradict this and did not prove it, because the bezel hides the edge of the glass. The 8
padding bytes after the pixel data, the same number M5GFX uses, probably leave room for the extra cycle.
A gray ramp shows nothing, since the whole picture moves together. FastEPD works around it only for
clears, with the hardware chip select off.

The bench examples that measured this are `lcd_clock_count`, `lcd_clock_pos` and `edge_stripes` in
`rust-m5paperdiy-examples`.

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
