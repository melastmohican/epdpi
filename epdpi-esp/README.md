# epdpi-esp

ESP32 buses for [`epdpi`](../epdpi), the `no_std` driver for raw-parallel e-paper displays.
They exist because the 16-gray waveforms need a row data phase near 16 us, which GPIO
bit-banging cannot reach.

| Bus | Row data phase | Needs |
| :--- | :--- | :--- |
| `fast_bus::Esp32s3Bus` | about 62 us | no feature, writes the S3 GPIO registers |
| `lcd_bus::LcdBus` | about 22 us | `esp32s3` feature, `esp-hal` 1.1, blocking API |

## Chips

Only the ESP32-S3 exists and is tested, on an M5Stack PaperDIY. The chip is a feature, as in
`esp-hal`.

```toml
epdpi = "0.1"
epdpi-esp = { version = "0.1", features = ["esp32s3"] }
```

`epdpi-esp` depends on `esp-hal` 1.1 (with its `unstable` feature). Cargo allows one copy of
`esp-hal`, so your project must use a compatible version.

## Async

`LcdBus` is blocking only for now. `fast_bus` works in both modes. For the async API turn off
default features on both crates.

## Status

See the `rust-m5paperdiy-examples` repository for bench tests of both buses.
