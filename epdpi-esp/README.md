# epdpi-esp

ESP32 buses for [`epdpi`](../epdpi), the `no_std` driver for raw-parallel e-paper displays.
They exist because the 16-gray waveforms are built for M5GFX's row data phase of 15.5 us
(computed: 248 bytes at 16 MHz), which GPIO bit-banging (about 250 us a row) cannot reach.

| Bus | Row data phase | Needs |
| :--- | :--- | :--- |
| `fast_bus::Esp32s3Bus` | about 62 us | no feature, writes the S3 GPIO registers |
| `lcd_bus::LcdBus` | about 22 us measured | `esp32s3` feature, `esp-hal` 1.1 |

The 6.5 us that `LcdBus` takes over the computed 15.5 us is the `send()` setup, which was not
isolated. A whole `LcdBus` row takes about 117 us (computed, not measured here), against about
91 us for M5GFX.

## Wiring

`LcdBus` expects CKV, LE, SPV, OE and PWR on plain GPIO pins, as on the M5Stack PaperDIY. A
board that routes them through a shift register or a power IC cannot use it and needs its
own `EpdParallelBus`. The LilyGo T5 4.7 S3 V2.4 is one: a 74HCT4094 carries STV, latch, OE,
mode and the panel rails, so its bus has a different edge sequence and power-on order.
`rust-lilygot5-47-s3-examples` has one, `LilygoBus`.

## Chips

Only the ESP32-S3 exists and is tested, on an M5Stack PaperDIY. The chip is a feature, as in
`esp-hal`.

```toml
epdpi = "0.1"
epdpi-esp = { version = "0.1", features = ["esp32s3"] }
```

`epdpi-esp` depends on `esp-hal` 1.1 (with its `unstable` feature). Cargo allows one copy of
`esp-hal`, so your project must use a compatible version.

`esp-hal` types are part of the public API (`Output`, `I8080`, the DMA types), so a new major
version of `esp-hal` needs a new version of `epdpi-esp` that is not compatible with this one. The
dependency is written `~1.1.0` for that reason: `epdpi-esp` 0.1 works with `esp-hal` 1.1.x only.

## Async

Both buses work in blocking and in async mode. For the async API turn off default features on
both crates. In async mode `LcdBus` awaits the DMA transfer of each row and takes a `DelayNs` for
its short waits, so the CPU is free between rows. On a PaperDIY an async refresh took about 5 to
40 percent longer than a blocking one, depending on the mode: the per-row await is a bigger share
when rows are cheap to build.

## Status

Tested on an M5Stack PaperDIY, in both modes. See the `rust-m5paperdiy-examples` repository (`blocking/` and `async/`, the bus diagnostics are on its
`bringup` branch) for tests of both buses.
