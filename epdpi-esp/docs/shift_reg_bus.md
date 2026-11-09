ESP32-S3 `LCD_CAM` i8080 bus with DMA for boards where a shift register carries the panel
control lines, the way the LilyGo T5 4.7 inch S3 (V2.4) is wired.

Verified on a V2.4 only, in both modes, with the examples in `rust-lilygot5-47-s3-examples`.
The bit layout of the register (`bits`) and the power-on order are the V2.4's, so another board
with a shift register but a different layout needs its own bus. The T5S3 Pro uses a different
power chip and other pins.

The data pins D0..D7 and the write strobe CL belong to the LCD peripheral, so the caller
configures an `I8080` with them and no chip select. CKV and SPH are plain GPIO. SPH is driven by
hand, low only while a row is sent (without it one pixel wide strokes ghost). Everything else
goes through a 74HCT4094, which speaks the 74HC595 protocol, on three more GPIO (data, clock,
store): the latch (LE), the gate start pulse (STV), the source output enable, the mode line and
the two panel rails with their disable line. The bits are in `bits`. Because those lines are
not pins, [`LcdBus`](crate::lcd_bus) cannot drive this board: the edge sequence, the order of
the rails and the row timing differ.

The frame and row sequence follow the vendor library's `ed047tc1.c` (read, not copied): CKV
idles low, OE and MODE are on only inside a frame, CKV is high for the row time (`ROW_US`,
22 us by default, `set_row_us` changes it) and the row goes out while it is high. The write
strobe must idle high (`ClockMode` with `Polarity::IdleHigh`, `Phase::ShiftLow`), as in the
vendor library. With the `esp-hal` default the data changes on the sampling edge and every
pixel that differs from its neighbours is wrong. GPIO46 is tied to the strobe net of the shift
register on this board: never drive it.

Works in both modes, like `LcdBus`. With the `blocking` feature (default) the I8080 is
`Blocking` and the delay is any `embedded_hal::delay::DelayNs`, such as
`esp_hal::delay::Delay`. Without it the I8080 is `Async`, a row waits for the DMA with
`wait_for_done`, and the delay is an `embedded_hal_async::delay::DelayNs`, such as
`embassy_time::Delay`. The row time itself is a busy wait in both modes. Needs the `esp32s3`
feature.

# Building these docs

docs.rs cannot build this module: `esp-hal` with `esp32s3` needs the Xtensa target, which docs.rs
does not have. This page is the prose only. The full API pages (`ShiftRegBus`, `ShiftRegBusError`,
`bits`) build locally with the `esp` toolchain:

```text
cargo +esp doc --no-deps --target xtensa-esp32s3-none-elf -Zbuild-std=core \
    -p epdpi-esp --features esp32s3 --open
```
