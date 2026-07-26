# Home telemetry broadcast

Arcadia Home telemetry is a watched-surface broadcast, not a polling loop. It provides a compact appliance view of current load while avoiding unnecessary work when the Home view is not visible.

## Product behavior

When the Home view is active and the browser document is visible, the browser opens `EventSource('/api/root/events')`. The service sends appliance-safe telemetry events. The browser renews its lease through `POST /api/root/events/renew`; the service records the current contact and expires the stream when renewal stops. When the user leaves Home, hides the document, or the stream fails, the browser closes the source and retains the last rendered telemetry as cached state. `/api/root` remains a one-shot snapshot fallback and retry bridge, not an interval-polling endpoint.

```text
Home active + visible
  -> open EventSource('/api/root/events')
  -> receive snapshot and lease
  -> renew lease with POST /api/root/events/renew before expiry
  -> receive periodic root telemetry while the lease is active
  -> receive heartbeat events for liveness
leave Home or hide document
  -> close EventSource immediately
  -> stop renewal and retry timers
  -> keep last telemetry rendered as cached state
  -> no interval polling remains running
SSE unsupported or errored
  -> fetch one /api/root snapshot, schedule stream retry, still Home-active + visible only
```

The Home card is an appliance card: compact load ring, 1m/5m/15m load bands, CPU/I/O/read/write chips, no raw operating-system file dump, developer log wall, or in-pane buttons.

## HTTP interface

```text
GET /api/root/events
Accept: text/event-stream
Content-Type: text/event-stream

POST /api/root/events/renew
Content-Type: application/json
{"leaseId":"home-load-..."}
```

The event route lives beside `/api/root` and reuses the same `ApiRootObject` builder. The renewal route records client contact through `lastContactUnix` and `expiresAtUnix`. `/api/root` remains the snapshot authority and one-shot fallback.

Event families:

```text
event: snapshot   # first full ApiRootObject
event: lease      # lease handle with lastContactUnix, renewAfterSeconds, expiresAtUnix
event: root       # periodic full ApiRootObject update while Home clients are subscribed
event: heartbeat  # lease and liveness readback
event: expired    # stream expired because renewal/contact stopped
```

## Implementation map

```text
Cargo.toml
  tokio feature: time
  async-stream
  futures-core

src/main.rs
  /api/root/events -> api_root_events_route
  /api/root/events/renew -> api_root_events_renew_route

src/bands/api_root.rs
  HomeTelemetryLease and HomeTelemetryLeaseResponse
  api_root_events_route
  api_root_events_renew_route
  api_root_object(&state) reused for snapshot/root events

static/app.js
  bindHomeLoadSubscription() owns the Home live surface
  EventSource('/api/root/events') is the preferred transport
  fetchSnapshotOnce() is a one-shot fallback only
  stopEvents() closes the source and clears timers on view or visibility transitions
```

Do not duplicate telemetry readers in the event route. The stream consumes the same root object surface as `/api/root` so the API remains canonical.

## Browser readback

The debug/readback object is intentionally small:

```js
window.arcadiaHomeLoadSubscriptionState
```

A healthy Home-active state includes `readyState: 1`, `hasSource: true`, a positive event count, `fallback: false`, and current lease timestamps. After leaving Home, it reports `hasSource: false` with no renewal or retry timer.

## Test contract

Tests guard both product shape and transport lifecycle:

- The Home section contains the load card, orb, spark bank, and telemetry grid.
- The Home section has no visible buttons or duplicate navigation controls.
- `static/app.js` opens `EventSource('/api/root/events')`, renews via `/api/root/events/renew`, and closes on view or visibility changes.
- Snapshot fallback uses `/api/root` without restoring Home-load interval polling.
- `src/main.rs` registers both telemetry routes.
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

## Local verification

Run the service in an environment you control, then inspect the stream on its configured listener:

```bash
curl -NsS -H 'Accept: text/event-stream' http://127.0.0.1:8080/api/root/events
```

Expected stream evidence includes `snapshot`, `lease`, `root`, `heartbeat`, `expired`, and `arcadia.api.root.v1`. To renew a lease:

```bash
curl -fsS -X POST http://127.0.0.1:8080/api/root/events/renew \
  -H 'Content-Type: application/json' \
  -d '{"leaseId":"<lease-from-event>"}'
```

In a browser, verify that Home active has a source, events arrive without polling, leaving Home closes the source, the card has no duplicate controls, and the browser console has no errors.
