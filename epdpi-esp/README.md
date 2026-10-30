# epdpi-esp

ESP32 buses for [`epdpi`](../epdpi), the `no_std` driver for raw-parallel e-paper displays.
They exist because the 16-gray waveforms need a row data phase near 16 us, which GPIO
bit-banging cannot reach.

| Bus | Row data phase | Needs |
| :--- | :--- | :--- |
| `fast_bus::Esp32s3Bus` | about 62 us | no feature, writes the S3 GPIO registers |
| `lcd_bus::LcdBus` | about 22 us | `esp32s3` feature, `esp-hal` 1.1 |

## Chips

Only the ESP32-S3 exists and is tested, on an M5Stack PaperDIY. The chip is a feature, as in
`esp-hal`.

```toml
epdpi = { git = "https://github.com/melastmohican/epdpi" }
epdpi-esp = { git = "https://github.com/melastmohican/epdpi", features = ["esp32s3"] }
```

`epdpi-esp` depends on `esp-hal` 1.1 (with its `unstable` feature). Cargo allows one copy of
`esp-hal`, so your project must use a compatible version.

## Async

Both buses work in blocking and in async mode. For the async API turn off default features on
both crates. In async mode `LcdBus` awaits the DMA transfer of each row and takes a `DelayNs` for
its short waits, so the CPU is free between rows. On a PaperDIY an async refresh took about 5 to
40 percent longer than a blocking one, depending on the mode: the per-row await is a bigger share
when rows are cheap to build.

## Status

Tested on an M5Stack PaperDIY, in both modes. See the `rust-m5paperdiy-examples` repository (blocking, the bus diagnostics are on its `bringup`
branch) and `rust-m5paperdiy-embassy-examples` (async) for tests of both buses.
