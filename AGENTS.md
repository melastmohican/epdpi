# epdpi

Workspace with two crates. Sibling of `../epdsi`; follow its conventions.

- `epdpi/`: `no_std` `embedded-hal` 1.0 driver for raw-parallel e-paper (ED047TC1 first). No
  vendor HAL dependency, ever.
- `epdpi-esp/`: ESP buses (`fast_bus` register-level, `lcd_bus` LCD_CAM with DMA). The chip is a
  feature, `esp32s3` is the only one. Only this crate may depend on `esp-hal`.

## Commands (match CI)

Run from the repo root. `default-members` is the core, so plain commands cover `epdpi` only.

```bash
cargo check --all-targets
cargo test --workspace                                       # both crates, host only
cargo test --no-default-features --features graphics --workspace   # async mode
cargo clippy -p epdpi --all-features -- -D warnings
cargo clippy -p epdpi-esp --all-targets -- -D warnings       # no chip feature, host
cargo clippy --no-default-features --features graphics -p epdpi -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
cargo check --target thumbv6m-none-eabi                      # RP2040 canary, core
cargo +esp check --target xtensa-esp32s3-none-elf -Zbuild-std=core -p epdpi-esp --features esp32s3
```

Never pass `--all-features` with `--workspace`: it turns on `esp32s3`, which needs the `esp`
toolchain. CI runs the thumbv6m and xtensa checks in blocking and async mode (add
`--no-default-features` for async). Add a target only when we own hardware for it.

## Rules

- Library code denies `unwrap`, `expect`, `panic` (see `[workspace.lints.clippy]`).
- Blocking and async share one body via `maybe_async_cfg`. Tests use the `epd_test!` macro in
  `epdpi/tests/support/mod.rs` (`epdpi-esp` has its own copy). Run both modes before pushing.
- `epdpi-esp` depends on `epdpi` with `default-features = false` and forwards `blocking`, so it
  never flips the core into blocking mode for an async user. Keep it that way.
- `esp-hal` needs its `unstable` feature for `lcd_cam` and `delay` (checked on 1.1.2).
- Hardware numbers (edge timing, delays, waveform matrix) come from FastEPD / M5GFX. Re-derive
  from there rather than from the original plan, which has known errors (LUT layout, 2bpp
  framebuffer, array length from an associated const). The plan is archived in
  `.agents/notes/original-design-plan.md` with its errors listed.
- Open work lives in `.agents/notes/backlog.md`. That folder is git-ignored by the global
  gitignore, like in `epdsi`, so it is a local working aid and not part of a clone.
- Docs read like a person wrote them: no em dashes, no "it's not X, it's Y".
- Never `git add .` in the parent `~/Src` repo. This directory is its own repo.
- Crate names `epdpi` and `epdpi-esp` were free on crates.io on 28 Oct 2026.
