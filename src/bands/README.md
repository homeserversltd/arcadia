# Arcadia Rust bands

Arcadia applies the infinite-infinite strut to the Rust service body.

`src/main.rs` is the thin process/router face. The band files here hold coherent transition surfaces and remain intentionally small enough to keep each responsibility inspectable.

`src/ui.rs` is a thin hoist into `bands/ui/` — ordered viewport child bands (`shell`, `home`, `sync`, `storage`, `local_ai`, `controllers`, `network`, `access_pin`, `updates`, `system`, `primitives`).

This tranche uses crate-root `include!` bands to preserve existing privacy and behavior while removing the 7k-line monolith. Later tranches may promote these bands into explicit Rust modules once each boundary has typed public interfaces.

## Home telemetry broadcast band

`api_root.rs` owns both the `/api/root` snapshot and the `/api/root/events` live stream. The stream is SSE-first because Home load telemetry is server-to-browser only.

Implementation rules:

- Keep `/api/root` as canonical snapshot authority.
- Register `/api/root/events` beside `/api/root` in `src/main.rs`.
- Register `/api/root/events/renew` as the client last-contact renewal route.
- Emit `snapshot`, `lease`, one-per-second `root`, `heartbeat`, and `expired` events.
- Keep `HOME_TELEMETRY_LEASES` as the server-side lease map until a broader hub is needed.
- Use `api_root_object(&state)` for the initial SSE `snapshot` and `/api/root` one-shot fallback.
- Use `api_root_telemetry_tick(&state)` for one-per-second SSE `root` events so ticks read `/proc` telemetry only.
- Do not duplicate telemetry readers outside `api_telemetry_node()`.
- Static browser code in `static/app.js` opens the stream only while Home is active and visible, renews the lease before expiry, and closes it on off-Home/hidden transitions.
- `/api/root` is a one-shot snapshot fallback and retry bridge only; do not restore interval polling for Home load telemetry.

Focused proof:

```bash
cargo test api_root_events -- --nocapture
cargo test home_view_is_operational_surface_without_duplicate_navigation -- --nocapture
cargo test api_root_routes_are_registered -- --nocapture
```
