# Arcadia Rust bands

Arcadia organizes the Rust service into small, inspectable bands.

`src/main.rs` is the thin process and router entry point. The files in this directory hold coherent application surfaces. `src/ui.rs` is a thin include spine into `bands/ui/`, whose ordered view bands cover the shell, Home, synchronization, storage, local AI, controllers, network, access PIN, updates, system view, and shared primitives.

The current crate-root `include!` arrangement preserves existing behavior while keeping the service's internal boundaries readable. A later refactor may promote bands to explicit Rust modules once each boundary has a typed public interface.

## Home telemetry broadcast band

`api_root.rs` owns both the `/api/root` snapshot and the `/api/root/events` live stream. The stream is SSE-first because Home telemetry travels from server to browser.

Implementation rules:

- Keep `/api/root` as the canonical snapshot route.
- Register `/api/root/events` and `/api/root/events/renew` in `src/main.rs`.
- Use the renewal route to record current client contact.
- Emit `snapshot`, `lease`, periodic `root`, `heartbeat`, and `expired` events.
- Keep the server-side lease map until a broader telemetry hub is needed.
- Reuse `api_root_object(&state)` for the initial SSE snapshot and the `/api/root` fallback.
- Use `api_root_telemetry_tick(&state)` for periodic SSE telemetry reads.
- Do not duplicate telemetry readers outside `api_telemetry_node()`.
- Open the browser stream only while Home is active and visible; renew before expiry and close it when the view becomes inactive or hidden.
- Keep `/api/root` as a one-shot fallback and retry bridge; do not restore interval polling for Home telemetry.

Focused proof:

```bash
cargo test api_root_events -- --nocapture
cargo test home_view_is_operational_surface_without_duplicate_navigation -- --nocapture
cargo test api_root_routes_are_registered -- --nocapture
```
