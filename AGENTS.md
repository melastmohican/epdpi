# epdpi

`no_std` `embedded-hal` 1.0 driver for raw-parallel e-paper (ED047TC1 first). Sibling of
`../epdsi`; follow its conventions.

## Commands (match CI)

```bash
cargo check --all-targets
cargo test --workspace
cargo test --no-default-features --features graphics   # async mode
cargo clippy --all-features --workspace -- -D warnings
cargo clippy --no-default-features --features graphics --workspace -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
```

## Rules

- Library code denies `unwrap`, `expect`, `panic` (see `[lints.clippy]`).
- Blocking and async share one body via `maybe_async_cfg`. Tests use the `epd_test!` macro in
  `tests/support/mod.rs`. Run both modes before pushing.
- Hardware numbers (edge timing, delays, waveform matrix) come from FastEPD / M5GFX. Re-derive
  from there rather than from the original plan, which has known errors (LUT layout, 2bpp
  framebuffer, array length from an associated const). The plan is archived in
  `.agents/notes/original-design-plan.md` with its errors listed.
- Open work lives in `.agents/notes/backlog.md`. That folder is git-ignored by the global
  gitignore, like in `epdsi`, so it is a local working aid and not part of a clone.
- Docs read like a person wrote them: no em dashes, no "it's not X, it's Y".
- Never `git add .` in the parent `~/Src` repo. This directory is its own repo.
