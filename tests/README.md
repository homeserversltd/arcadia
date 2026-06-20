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

## Home telemetry broadcast tests

The Home load broadcast contract is pinned in `tests/bands/home_sync_storage.rs`.

The focused tests are:

```bash
cargo test api_root_events -- --nocapture
cargo test home_view_is_operational_surface_without_duplicate_navigation -- --nocapture
cargo test api_root_routes_are_registered -- --nocapture
```

They guard:

- `/api/root/events` route registration and `/api/root/events/renew` lease-renewal route registration.
- SSE source dependencies and `api_root_events_route` shape.
- `snapshot`, `lease`, `root`, `heartbeat`, and `expired` event families.
- Browser `EventSource('/api/root/events')` lifecycle.
- client renewal through `/api/root/events/renew`.
- `/api/root` one-shot snapshot fallback as compatibility only, with no Home telemetry interval polling.
- Home-active and document-visible guards for any telemetry transport.
- source closure on `arcadia:view-change` and `visibilitychange`.
- zero Home buttons and no duplicate navigation controls.
