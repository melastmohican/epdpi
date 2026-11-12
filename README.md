# epdpi

Drivers for raw-parallel e-paper displays, where the MCU drives the panel's gate and source
drivers over an 8-bit bus and runs the waveform itself. `no_std`, `embedded-hal` 1.0, blocking
or async.

| Crate | What |
| :--- | :--- |
| [`epdpi`](epdpi) | The driver: panels, waveforms, frame buffer, portable GPIO bus. No vendor HAL. |
| [`epdpi-esp`](epdpi-esp) | Faster buses for the ESP32-S3: LCD_CAM with DMA, and register-level GPIO. |

```toml
epdpi = "0.3"
epdpi-esp = { version = "0.3", features = ["esp32s3"] }   # ESP32-S3 only
```

Tested on an M5Stack PaperDIY and a LilyGo T5 4.7 S3 V2.4, both with the ED047TC1 panel. See the crate READMEs for status.
Examples for the LilyGo live in [`rust-lilygot5-47-s3-examples`](https://github.com/melastmohican/rust-lilygot5-47-s3-examples). Ports of
the Arduino sketches live in [`rust-m5paperdiy-examples`](https://github.com/melastmohican/rust-m5paperdiy-examples),
the blocking ones in its `blocking/` folder and the async twins in `async/` (the bring-up diagnostics are on its `bringup` branch).

Licensed under the GNU General Public License v3.0 only (see [`LICENSE`](LICENSE)).
