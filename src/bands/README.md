# Arcadia Rust bands

Arcadia applies the infinite-infinite strut to the Rust service body.

`src/main.rs` is the thin process/router face. The band files here hold coherent transition surfaces and remain intentionally small enough to keep each responsibility inspectable.

This tranche uses crate-root `include!` bands to preserve existing privacy and behavior while removing the 7k-line monolith. Later tranches may promote these bands into explicit Rust modules once each boundary has typed public interfaces.

## Home telemetry broadcast band

`api_root.rs` owns both the `/api/root` snapshot and the `/api/root/events` live stream. The stream is SSE-first because Home load telemetry is server-to-browser only.

Implementation rules:

- Keep `/api/root` as canonical snapshot authority.
- Register `/api/root/events` beside `/api/root` in `src/main.rs`.
- Emit `snapshot`, `lease`, `root`, and `heartbeat` events.
- Reuse `api_root_object(&state)` for streamed state; do not duplicate `/proc`, load, disk, or telemetry readers in a second path.
- Keep route-local streaming acceptable until a real multi-topic `HomeTelemetryHub` is needed.
- Static browser code in `static/app.js` opens the stream only while Home is active and visible, and closes it on off-Home/hidden transitions.
- Polling `/api/root` is fallback only and must retain the same Home-active/visible guard.

Focused proof:

```bash
cargo test api_root_events -- --nocapture
cargo test home_view_is_operational_surface_without_duplicate_navigation -- --nocapture
cargo test api_root_routes_are_registered -- --nocapture
```
