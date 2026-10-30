# epdpi

Drivers for raw-parallel e-paper displays, where the MCU drives the panel's gate and source
drivers over an 8-bit bus and runs the waveform itself. `no_std`, `embedded-hal` 1.0, blocking
or async.

| Crate | What |
| :--- | :--- |
| [`epdpi`](epdpi) | The driver: panels, waveforms, frame buffer, portable GPIO bus. No vendor HAL. |
| [`epdpi-esp`](epdpi-esp) | Faster buses for the ESP32-S3: LCD_CAM with DMA, and register-level GPIO. |

```toml
epdpi = { git = "https://github.com/melastmohican/epdpi" }
epdpi-esp = { git = "https://github.com/melastmohican/epdpi", features = ["esp32s3"] }   # ESP32-S3 only
```

Not on crates.io yet, so depend on the repository for now.

Tested on an M5Stack PaperDIY (ED047TC1). See the crate READMEs for status. Ports of
the Arduino sketches live in [`rust-m5paperdiy-examples`](https://github.com/melastmohican/rust-m5paperdiy-examples),
the blocking ones in its `blocking/` folder and the async twins in `async/` (the bring-up diagnostics are on its `bringup` branch).

Licensed under the GNU General Public License v3.0 only (see [`LICENSE`](LICENSE)).
