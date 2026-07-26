# Arcadia Rust test bands

This root-level test band holds Arcadia Rust contract tests.

Production source lives under `src/bands/`. Tests remain at repository root so production bands stay focused on application behavior while the test harness can inspect the binary crate's private surfaces.

`index.rsi` is intentionally not `index.rs`: Cargo treats `tests/*.rs` as standalone integration crates, while these contract tests are included inside the Arcadia binary crate.

The repository configures Cargo with one job and one test thread because some tests exercise host-state readers. Run focused work first, then the bounded suite:

```bash
cargo test --tests -- --nocapture
```

## Home telemetry broadcast tests

The Home telemetry contract is pinned in `tests/bands/home_sync_storage.rs`.

Focused tests:

```bash
cargo test api_root_events -- --nocapture
cargo test home_view_is_operational_surface_without_duplicate_navigation -- --nocapture
cargo test api_root_routes_are_registered -- --nocapture
```

They guard:

- `/api/root/events` and `/api/root/events/renew` route registration.
- SSE source dependencies and `api_root_events_route` behavior.
- `snapshot`, `lease`, `root`, `heartbeat`, and `expired` event families.
- Browser `EventSource('/api/root/events')` lifecycle and renewal.
- `/api/root` as a one-shot compatibility fallback rather than interval polling.
- Home-active and document-visible guards for telemetry transport.
- Source closure on view and visibility changes.
- A Home card with no duplicate navigation controls.
