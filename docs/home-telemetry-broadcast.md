---
id: home-telemetry-broadcast
type: doctrine
status: active
locus: arcadia/home/telemetry-broadcast
aliases:
  - home load broadcast
  - arcadia sse telemetry
  - api root events
  - eventsource home telemetry
description: Arcadia Home telemetry broadcast contract for /api/root/events, EventSource lifecycle, fallback-only polling, and live verification.
---

# Home Telemetry Broadcast

Arcadia Home load telemetry is a watched-surface broadcast, not a raw polling loop.

When the Home viewport is active and the browser document is visible, the browser joins the live Home load feed with `EventSource('/api/root/events')`. Rust/Axum keeps an HTTP SSE stream open and sends appliance-safe telemetry events. When the operator leaves Home, hides the document, or the stream fails, the browser closes the source. `/api/root` polling is retained only as a bounded compatibility fallback.

## Product contract

```text
Home active + visible
  -> open EventSource('/api/root/events')
  -> receive snapshot and lease
  -> receive root telemetry events every cadence
  -> receive heartbeat events for liveness/readback
leave Home or hide document
  -> close EventSource immediately
  -> no interval polling remains running
SSE unsupported or errored
  -> fallback to /api/root polling, still Home-active + visible only
```

The Home card remains an appliance card: compact load ring, 1m/5m/15m load bands, CPU/I/O/read/write chips, no raw `/proc` copy, no developer log wall, no in-pane buttons.

## Server route

```text
GET /api/root/events
Accept: text/event-stream
Content-Type: text/event-stream
```

The route lives beside `/api/root` and reuses the same `ApiRootObject` builder. `/api/root` remains the snapshot authority and fallback surface.

Current event families:

```text
event: snapshot   # first full ApiRootObject
event: lease      # lease-ish membership/readback for topic home.load
event: root       # periodic full ApiRootObject update
event: heartbeat  # liveness/readback for the current lease
```

The first implementation uses a monotonic lease id and heartbeat payloads. A later tranche may promote this into a full `HomeTelemetryHub` with server-side active lease registry, timeout reaper, and diagnostics.

## Rust seams

```text
Cargo.toml
  tokio feature: time
  async-stream
  futures-core

src/main.rs
  axum::response::sse::{Event, KeepAlive, Sse}
  futures_core::Stream
  std::convert::Infallible
  /api/root/events -> api_root_events_route

src/bands/api_root.rs
  HOME_TELEMETRY_LEASE_COUNTER
  api_root_events_route
  api_root_object(&state) reused for snapshot/root events
```

Do not duplicate telemetry readers in the event route. The event stream consumes the same root object surface as `/api/root` so the API shape stays canonical.

## Browser seams

```text
static/app.js
  bindHomeLoadPolling() owns current compatibility name
  EventSource('/api/root/events') is preferred transport
  snapshot/root events call the existing Home load DOM apply path
  lease/heartbeat are stored in window.arcadiaHomeLoadPollState for readback
  startPolling() is fallback only
  stopEvents() closes the source on view/visibility transitions
```

The public debug/readback object is intentionally small:

```js
window.arcadiaHomeLoadPollState
```

Healthy Home-active readback:

```js
{
  readyState: 1,
  hasSource: true,
  events: 5,
  polls: 0,
  fallback: false,
  lease: 'lease',
  heartbeat: 'heartbeat'
}
```

After switching away from Home:

```js
{
  hasSource: false,
  timer: false,
  polls: 0
}
```

## Test contract

The Arcadia tests must guard both product shape and transport lifecycle:

- Home section has `load-home-card`, `load-orb`, `load-spark-bank`, and `load-telemetry-grid`.
- Home section has zero visible buttons and no duplicate navigation controls.
- `static/app.js` contains `new EventSource('/api/root/events')`.
- `static/app.js` closes the source on `arcadia:view-change` and `visibilitychange`.
- fallback polling still contains `fetch('/api/root')` and `setInterval(poll, pollMs)`.
- fallback polling is guarded by `[data-view-panel="home"].is-active` and `document.visibilityState === 'visible'`.
- `src/main.rs` registers `.route("/api/root/events", get(api_root_events_route))`.
- `src/bands/api_root.rs` emits `snapshot`, `lease`, `root`, and `heartbeat` events.

Focused proof commands:

```bash
cargo fmt --check
cargo test api_root_events -- --nocapture
cargo test home_view_is_operational_surface_without_duplicate_navigation -- --nocapture
cargo test api_root_routes_are_registered -- --nocapture
cargo test
cargo build --release
```

## Live verification

After Cibation admission, rebuild current `origin/main`, deploy with Harmonia, then prove the live body:

```bash
curl -NsS -H 'Accept: text/event-stream' http://127.0.0.1:8080/api/root/events
```

Expected stream evidence:

```text
event: snapshot
event: lease
event: root
event: heartbeat
arcadia.api.root.v1
```

Browser proof on `http://console.home.arpa/` or the target IP should show:

- Home active: `hasSource: true`, `readyState: 1`, `events > 0`, `polls: 0`, `fallback: false`.
- After switching away from Home: `hasSource: false`, timer absent, polls still `0`.
- Home load card has zero buttons, no overflow, and no JavaScript console errors.

## Paligenesis authority

Canonical doctrine lives in Paligenesis leaf:

```text
arcadia-home-telemetry-broadcast-north-star
locus: workflow/arcadia/home-telemetry-broadcast
```

Use that leaf when judging whether future work preserves the broadcast architecture. Polling is a fallback bridge, not the named product architecture.
