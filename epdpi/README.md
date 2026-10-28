# epdpi

`no_std`, `embedded-hal` 1.0 driver framework for raw-parallel e-paper displays, where
the MCU drives the panel's gate and source drivers over an 8-bit bus and runs the
waveform itself. Blocking by default, async with `default-features = false`.

It is the parallel counterpart of [`epdsi`](https://github.com/melastmohican/epdsi),
which covers SPI panels that have their own controller IC.

| Panel | Resolution | Grays | Boards |
| :--- | :--- | :--- | :--- |
| `Ed047tc1` | 960 x 540 | 16 | M5Stack PaperDIY |

## Status

The sequencing, waveform packing and pin behavior are covered by host tests in both
API modes. On a M5Stack PaperDIY, `Clear` and `FastMono` (no full-screen flash) run correctly on
hardware. `Quality16` runs, but the top gray levels look alike. `M5Quality`, which follows
M5GFX's waveform, gives an even ramp over an `LCD_CAM` DMA bus, with midtones about 20 levels
darker than M5GFX, and a squashed ramp over GPIO bit-banging. That DMA bus is
[`epdpi-esp::lcd_bus`](../epdpi-esp). `rust-m5paperdiy-examples` holds the bench test suite.

## Usage

```rust,ignore
use epdpi::prelude::*;

let bus = GenericGpioBus::new(
    [d0, d1, d2, d3, d4, d5, d6, d7],
    ControlPins { ckv, spv, sph, oe, le, cl, pwr },
    delay,
);
let mut display = EpdDisplayBuilder::<Ed047tc1>::new().build(bus, &mut prev, &mut line)?;
let mut fb = EpdFrameBuffer::<Ed047tc1>::new(&mut fb_mem)?;

display.clear()?;
fb.fill(0xF);
// draw with embedded-graphics, Gray4 colors
display.refresh(&fb, RefreshMode::Quality16)?;
```

Memory: `prev` and `fb_mem` are 259,200 bytes each (4 bits per pixel), `line` is 248 bytes.
On the ESP32-S3 put the two big ones in PSRAM.

## Refresh modes

| Mode | What it does | Flashes |
| :--- | :--- | :--- |
| `Clear` | 8 darken passes, 8 lighten passes, discharge | yes |
| `Quality16` | clear, then the 8-pass 16-gray waveform | yes |
| `FastMono` | 4 passes, drives only pixels that changed, threshold at gray 8 | no |

The ED047TC1 waveform matrix indexes by target gray only, so `Quality16` always clears
first. `FastMono` ghosts over time, so interleave a `Clear`.

## Buses

`GenericGpioBus` bit-bangs any `OutputPin`. All 15 pins must share one type, which is
how `esp-hal`, `embassy-rp` and `embassy-stm32` expose them. A DMA bus (ESP32-S3
`LCD_CAM`) implements `EpdParallelBus` in the same five calls.

Board power is outside this crate. The PaperDIY's M5PM1 PMIC has to switch the panel
rail on before `set_power(true)`.

## Features

- `blocking` (default): `embedded-hal` API. Off gives the `embedded-hal-async` API.
- `graphics` (default): `DrawTarget<Color = Gray4>` on `EpdFrameBuffer`.
- `defmt`: `defmt::Format` on public types.

## References

Edge sequences, delays and the waveform matrix follow
[FastEPD](https://github.com/bitbank2/FastEPD) (`PaperS3RowControl`, `u8M5Matrix`) and
[M5GFX](https://github.com/m5stack/M5GFX).

## License

MIT OR Apache-2.0
