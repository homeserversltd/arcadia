# Arcadia

Arcadia is the HomeServer console GUI: a Rust web application for viewing and managing a HomeServer appliance from a browser.

## What it provides

- A responsive appliance dashboard for system status, storage, network, updates, games, and local AI.
- An Axum HTTP service with health and status routes.
- A theme system built from validated JSON theme files.
- Server-Sent Events for live Home telemetry when the Home view is visible.

## Run and build

Run a development instance:

```bash
cargo run
```

Build a release binary:

```bash
cargo build --release
```

For a production installation, place the release binary under your chosen application directory and manage it with your platform's service manager. Arcadia can serve native Rust TLS directly for `https://console.home.arpa/`; certificate and key material are always supplied by the deployment environment, never embedded in the binary. See [native HTTPS serving](docs/https-serving.md).

## Interface overview

- `/` serves the console.
- `/health` provides a basic health response.
- `/api/status` provides the current appliance status.
- `/static/app.css` and `/static/app.js` serve the browser assets.
- `/api/root` provides a Home telemetry snapshot.
- `/api/root/events` and `/api/root/events/renew` provide the optional Home telemetry stream and lease renewal interface.

## Themes and UI composition

Theme files live in `static/themes/`. Each JSON filename becomes a selectable theme name and must provide the complete token set documented in `static/themes/README.md`. `build.rs` validates theme files at build time.

Shared UI geometry and responsive rules live in `static/ux/`. `static/ux/arcadia-ux.css` defines reusable shell, control, spacing, and text primitives; `static/ux/arcadia-viewports.css` owns viewport-specific layout. `static/app.css` consumes those primitives for feature-specific composition and should not add viewport media rules.

The served stylesheet is assembled in this order: generated theme CSS, shared UX CSS, application defaults, and viewport CSS.

## System and telemetry contracts

- `docs/system-appliance-front-panel.md` describes the System view's appliance-oriented interaction model.
- `docs/home-telemetry-broadcast.md` describes the Home telemetry snapshot, event stream, browser lifecycle, and verification contract.

## Security boundary

Do not commit PINs, credentials, tokens, private keys, private certificates, personal data, game media, save data, or environment-specific deployment records. Configure those values only in the deployment environment.
