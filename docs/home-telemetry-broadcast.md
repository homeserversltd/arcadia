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
description: Arcadia Home telemetry broadcast contract for /api/root/events, renewal leases, EventSource lifecycle, snapshot fallback, and live verification.
---

# Home Telemetry Broadcast

Arcadia Home load telemetry is a watched-surface broadcast, not a raw polling loop.

When the Home viewport is active and the browser document is visible, the browser joins the live Home load feed with `EventSource('/api/root/events')`. Rust/Axum keeps an HTTP SSE stream open and sends appliance-safe telemetry events. The browser renews the lease through `POST /api/root/events/renew`; the server records `lastContactUnix` and `expiresAtUnix`, and the stream expires when renewal stops. When the operator leaves Home, hides the document, or the stream fails, the browser closes the source and keeps the last rendered telemetry as a cache. `/api/root` is retained only as a one-shot snapshot fallback and retry bridge, not as interval polling.

## Product contract

```text
Home active + visible
  -> open EventSource('/api/root/events')
  -> receive snapshot and lease
  -> renew lease with POST /api/root/events/renew before expiresAtUnix
  -> receive root telemetry events every cadence
  -> receive heartbeat events for liveness/readback
leave Home or hide document
  -> close EventSource immediately
  -> stop renewal timer and retry timer
  -> keep last telemetry rendered as cached state
  -> no interval polling remains running
SSE unsupported or errored
  -> fetch one /api/root snapshot, schedule stream retry, still Home-active + visible only
```

The Home card remains an appliance card: compact load ring, 1m/5m/15m load bands, CPU/I/O/read/write chips, no raw `/proc` copy, no developer log wall, no in-pane buttons.

## Server route

```text
GET /api/root/events
Accept: text/event-stream
Content-Type: text/event-stream

POST /api/root/events/renew
Content-Type: application/json
{"leaseId":"home-load-..."}
```

The event route lives beside `/api/root` and reuses the same `ApiRootObject` builder. The renewal route is the client contact surface: each valid renewal updates server-side `lastContactUnix` and `expiresAtUnix`. `/api/root` remains the snapshot authority and one-shot fallback surface.

Current event families:

```text
event: snapshot   # first full ApiRootObject
event: lease      # server membership handle with lastContactUnix, renewAfterSeconds, expiresAtUnix
event: root       # periodic full ApiRootObject update
event: heartbeat  # server-side lease/liveness readback
event: expired    # server expired the stream because renewal/contact stopped
```

The current implementation uses a monotonic lease id, a server-side lease map, renewal POSTs, heartbeat payloads, and expiry checks. A later tranche may promote this into a full multi-topic `HomeTelemetryHub` with richer active lease diagnostics.

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
  /api/root/events/renew -> api_root_events_renew_route

src/bands/api_root.rs
  HOME_TELEMETRY_LEASE_COUNTER
  HOME_TELEMETRY_LEASES
  HomeTelemetryLease and HomeTelemetryLeaseResponse
  api_root_events_route
  api_root_events_renew_route
  api_root_object(&state) reused for snapshot/root events
```

Do not duplicate telemetry readers in the event route. The event stream consumes the same root object surface as `/api/root` so the API shape stays canonical.

## Browser seams

```text
static/app.js
  bindHomeLoadSubscription() owns the Home load live surface
  EventSource('/api/root/events') is the preferred transport
  /api/root/events/renew is the client renewal/contact route
  snapshot/root events call the existing Home load DOM apply path
  lease/heartbeat are stored in window.arcadiaHomeLoadSubscriptionState for readback
  cachedRoot preserves the last telemetry values while moving between pages
  fetchSnapshotOnce() is a one-shot fallback only
  stopEvents() closes the source and clears renewal/retry timers on view/visibility transitions
```

The public debug/readback object is intentionally small:

```js
window.arcadiaHomeLoadSubscriptionState
```

Healthy Home-active readback:

```js
{
  readyState: 1,
  hasSource: true,
  events: 5,
  fallbackSnapshots: 0,
  fallback: false,
  lastContactUnix: 1781990000,
  expiresAtUnix: 1781990030,
  lease: 'homeTelemetryLease',
  heartbeat: 'homeTelemetryLease'
}
```

After switching away from Home:

```js
{
  hasSource: false,
  renewalTimer: false,
  retryTimer: false,
  fallbackSnapshots: 0
}
```

## Test contract

The Arcadia tests must guard both product shape and transport lifecycle:

- Home section has `load-home-card`, `load-orb`, `load-spark-bank`, and `load-telemetry-grid`.
- Home section has zero visible buttons and no duplicate navigation controls.
- `static/app.js` contains `new EventSource('/api/root/events')`.
- `static/app.js` renews the lease through `fetch('/api/root/events/renew')`.
- `static/app.js` closes the source on `arcadia:view-change` and `visibilitychange`.
- fallback snapshot logic contains `fetch('/api/root')` but no Home load `setInterval(poll, pollMs)`.
- fallback snapshot/retry logic is guarded by `[data-view-panel="home"].is-active` and `document.visibilityState === 'visible'`.
- legacy `bindHomeLoadPolling`, `arcadiaHomeLoadPollState`, and `arcadiaHomeLoadPolling` names are absent.
- `src/main.rs` registers `.route("/api/root/events", get(api_root_events_route))` and `.route("/api/root/events/renew", post(api_root_events_renew_route))`.
- `src/bands/api_root.rs` emits `snapshot`, `lease`, `root`, `heartbeat`, and `expired` events.

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
event: expired
arcadia.api.root.v1
```

Lease renewal proof:

```bash
curl -fsS -X POST http://127.0.0.1:8080/api/root/events/renew \
  -H 'Content-Type: application/json' \
  -d '{"leaseId":"<lease-from-event>"}'
```

Expected renewal evidence includes `kind: homeTelemetryLease`, `lastContactUnix`, `renewAfterSeconds`, and `expiresAtUnix`. Expired or unknown leases return `410 Gone` with `kind: homeTelemetryLeaseExpired`.

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
