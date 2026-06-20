# Arcadia Rust test bands

Root-level test band for Arcadia Rust contract tests.

The production source spine lives under `src/bands/`. Test decomposition belongs here at repository root so source bands remain production surfaces and test contracts stay in the root test harness.

`index.rsi` is intentionally not `index.rs`: Cargo treats `tests/*.rs` as standalone integration crates, while these contract tests must be included inside the Arcadia binary crate to inspect private appliance surfaces.
