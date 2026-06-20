# Arcadia Rust test bands

Root-level test band for Arcadia Rust contract tests.

The production source spine lives under `src/bands/`. Test decomposition belongs here at repository root so source bands remain production surfaces and test contracts stay in the root test harness.

`index.rsi` is intentionally not `index.rs`: Cargo treats `tests/*.rs` as standalone integration crates, while these contract tests must be included inside the Arcadia binary crate to inspect private appliance surfaces.

The repository carries `.cargo/config.toml` with `jobs = 1` and `RUST_TEST_THREADS = 1` because these tests exercise appliance-render/state contracts that touch host-state readers. The safe default proof lane is therefore bounded even when an operator runs:

```bash
CARGO_HOME=/var/cache/fulcrum/cargo/uid-$(id -u) \
CARGO_TARGET_DIR=/var/cache/fulcrum/arcadia-target \
cargo test --tests -- --nocapture
```

Focused work should still run the narrow named test first, then the bounded suite once.
