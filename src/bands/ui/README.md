# Arcadia UI bands

`src/ui.rs` is a thin include spine. The ordered UI bands live here:

- `mod.rs` — imports, views, layout, and include spine.
- `shell.rs` — header, sidebar, and theme controls.
- `primitives.rs` — shared buttons, rows, and helpers.
- Viewport bands — one focused file for each major console view.

Run the Arcadia test suite from the repository root to verify UI contracts:

```bash
cargo test
```
