# Arcadia UI bands

`src/ui.rs` is a thin hoist. The ordered child spine lives here.

- `mod.rs` — imports, `VIEWS`, `layout`, include spine
- `shell.rs` — header, sidebar, theme chips
- `primitives.rs` — shared buttons, rows, helpers
- Viewport bands — one file per focused viewport

Proof: `cargo test` in the arcadia worktree.
