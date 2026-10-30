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

The sequencing, waveform packing, pin behavior, window updates and rotation are covered by host
tests in both API modes. On a M5Stack PaperDIY every refresh mode runs on hardware over the
`LCD_CAM` DMA bus of [`epdpi-esp`](../epdpi-esp), in blocking and in async mode. `M5Quality` gives
an even 16-step ramp like M5GFX's `epd_quality`, with levels 7 to 9 about 0.1 of the gray range
darker than M5GFX. `Quality16` runs, but its top gray levels look alike. Over GPIO the rows take
too long for the 16-gray waveforms (about 250 us against the 16 us they assume), so the ramp
squashes into a few tones. `rust-m5paperdiy-examples` holds the bench tests and the ports of the
Arduino sketches.

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

| Mode | What it does | Notes |
| :--- | :--- | :--- |
| `Clear` | 8 darken passes, 8 lighten passes, discharge | flashes |
| `Quality16` | clear, then the 8-pass 16-gray waveform | flashes, several seconds |
| `FastMono` | 4 passes, drives only pixels that changed, threshold at gray 8 | no full-screen flash, 379 ms |
| `M5Quality` | M5GFX `epd_quality`: a short erase toward mid gray, then the 16-gray waveform for the target | no clear, needs the previous picture |
| `M5Text` | M5GFX `epd_text`: the erase, then 12 passes, only for pixel pairs that are not white before and after | a page of text flashes less than `M5Quality` |
| `M5Fast` | M5GFX `epd_fast`: black and white, 8 passes, only pixel pairs that changed | |
| `M5Fastest` | M5GFX `epd_fastest`: like `M5Fast` with 5 passes | |

The ED047TC1 waveform matrix indexes by target gray only, so `Quality16` always clears
first. `FastMono` ghosts over time, so interleave a `Clear`. The `M5*` modes follow the previous
picture the display records after every refresh, so they need no clear.

Full-screen times on a PaperDIY over `LcdBus`, blocking, for a page of text and gray cells in
portrait: `M5Quality` 955 ms, `M5Text` 633 ms, `M5Fast` 396 ms, `M5Fastest` 265 ms. The async API
is about 5 to 40 percent slower, because every row awaits the DMA.

`WaveformEngine::m5_scanline` builds one row of an `M5*` mode. A caller that builds many rows of
one pass can make an `M5Pass` once and call `M5Pass::scanline` for each row, which is what
`EpdDisplay` does.

## Partial updates

`EpdDisplay::refresh_rect(&fb, mode, rect)` refreshes one `Rect`, and
`refresh_dirty(&mut fb, mode)` refreshes the area drawn since its last call. `EpdFrameBuffer`
keeps that area (`dirty`, `mark_dirty`, `clear_dirty`), and `set_pixel`, `fill` and the
`DrawTarget` update it. A `Rect` is clipped to the panel and widened to whole pixel pairs.
`Clear` and `Quality16` always refresh the whole screen.

Every row is still clocked, but rows outside the window hold and skip the waveform work, so a
small window is faster than a full update with the same mode. On a PaperDIY a 110 x 67 pixel
border update with `M5Fastest` takes 116 ms, against 265 ms for the whole screen.

## Orientation

The panel is wired landscape, 960 x 540. `EpdFrameBuffer::set_rotation(Rotation)` turns the
picture that the `embedded-graphics` `DrawTarget` draws. `Deg90` and `Deg270` give a 540 x 960
portrait, and `Deg270` puts the panel connector at the bottom, like M5GFX's default on the
PaperDIY. `set_pixel`, `pixel`, the dirty area and `as_bytes` stay in panel coordinates, and
`logical_size`, `set_logical_pixel` and `rect_to_panel` convert.

## Buses

`GenericGpioBus` bit-bangs any `OutputPin`. All 15 pins must share one type, which is
how `esp-hal`, `embassy-rp` and `embassy-stm32` expose them. A DMA bus implements
`EpdParallelBus` in the same five calls, and `epdpi-esp` has one for the ESP32-S3 `LCD_CAM`.

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
