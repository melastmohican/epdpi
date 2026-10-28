# epdpi

Drivers for raw-parallel e-paper displays, where the MCU drives the panel's gate and source
drivers over an 8-bit bus and runs the waveform itself. `no_std`, `embedded-hal` 1.0, blocking
or async.

| Crate | What |
| :--- | :--- |
| [`epdpi`](epdpi) | The driver: panels, waveforms, frame buffer, portable GPIO bus. No vendor HAL. |
| [`epdpi-esp`](epdpi-esp) | Faster buses for the ESP32-S3: LCD_CAM with DMA, and register-level GPIO. |

```toml
epdpi = "0.1"
epdpi-esp = { version = "0.1", features = ["esp32s3"] }   # ESP32-S3 only
```

Tested on an M5Stack PaperDIY (ED047TC1). See the crate READMEs for status. Bench tests live in
`rust-m5paperdiy-examples`.

Licensed under either of Apache License 2.0 or MIT at your option.
