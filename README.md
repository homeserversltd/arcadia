# Arcadia

Arcadia is the pure Rust HomeConsole control surface served on the Arch gaming console.

Current scaffold:

- `axum` backend
- default bind: `0.0.0.0:8080`
- canonical URL through console nftables: `http://console.example.com/`
- routes:
  - `/`
  - `/health`
  - `/api/status`
  - `/static/app.css`
  - `/static/app.js`

Theme doctrine:

- Arcadia theme JSONs live at `static/themes/*.json`.
- Inside a Cibation worktree they live at `static/themes/*.json (in any checkout or worktree)`.
- The filename stem is the theme name propagated through generated CSS, generated JavaScript, `<html data-theme>`, localStorage, and the header theme button.
- Each theme is one flat JSON singleton containing the complete variable set documented in `static/themes/README.md`.
- `build.rs` validates the singleton JSONs at compile time and fails the build if a theme omits a required variable.


UX library doctrine:

- Arcadia shared UX CSS lives at `static/ux/`.
- Arcadia UI work starts in the shared UX library and theme system. The library is the reliable path for appliance geometry, control rhythm, responsive behavior, and themeable surfaces.
- `static/ux/arcadia-ux.css` owns reusable shell rhythm, component scale, action/control tracks, spacing, utility primitives, and ordinary text bounds.
- `static/ux/arcadia-viewports.css` owns all tablet/phone viewport dialing; `static/app.css` must not grow new `@media` bands.
- The served `/static/app.css` response is composed by Rust in this order: generated theme CSS, UX library CSS, app defaults, viewport CSS.
- Theme JSONs remain skin/color/radius inputs only; responsive layout belongs in the UX library.
- Feature selectors in `static/app.css` consume UX/theme variables for domain-specific composition. When button, card, row, or viewport geometry drifts, repair the shared track or its viewport-scoped consumption before adding local one-off sizing.

System appliance doctrine:

- `docs/system-appliance-front-panel.md` governs the System viewport.
- System keeps power, remote access, secure web access, service health, and diagnostics capabilities, but presents them as a HomeConsole appliance front panel.
- One job has one visible control; implementation duplicate nouns, permanent enable/disable pairs, and empty log/copy/download buttons are forbidden.
- Raw service evidence belongs behind Diagnostics, not as front-panel button spam.

Home telemetry broadcast doctrine:

- `docs/home-telemetry-broadcast.md` governs the Home load live telemetry substrate.
- `/api/root` remains the snapshot object tree; `/api/root/events` is the Server-Sent Events stream for watched Home telemetry.
- Browser lifecycle is Home-active and visible only: open `EventSource('/api/root/events')` on Home, renew through `POST /api/root/events/renew`, close it when leaving Home or hiding the document.
- `/api/root` is a one-shot snapshot fallback/retry bridge only when SSE is unavailable or errored; no Home load interval polling remains.
- The stream emits `snapshot`, `lease`, `root`, `heartbeat`, and `expired` events and reuses `ApiRootObject` rather than duplicating telemetry readers.

Run locally:

```bash
cargo run
```

Build release:

```bash
cargo build --release
```

Local development:

```bash
cargo run
```

Deployment is environment-specific. Build with `cargo build --release`, install the binary using your platform service manager, and set `ARCADIA_BIND` and `ARCADIA_CANONICAL_URL` for the target host.

Runtime service target:

```text
ExecStart=/usr/local/bin/arcadia
Environment=ARCADIA_BIND=0.0.0.0:8080
```

Public-safe boundary: do not commit PINs, Keyman material, ROMs, BIOS files, save data, private receipts, provider credentials, or household secrets.
