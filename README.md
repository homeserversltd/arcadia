# Arcadia

Arcadia is the pure Rust HomeConsole control surface served on the Arch gaming console.

Current scaffold:

- `axum` backend
- default bind: `0.0.0.0:8080`
- canonical URL through console nftables: `http://console.home.arpa/`
- routes:
  - `/`
  - `/health`
  - `/api/status`
  - `/static/app.css`
  - `/static/app.js`

Theme doctrine:

- Arcadia theme JSONs live at `/fulcrum/attachments/arcadia/static/themes/*.json`.
- Inside a Cibation worktree they live at `/fulcrum/attachments/arcadia/.worktrees/<work-id-or-task>/static/themes/*.json`.
- The filename stem is the theme name propagated through generated CSS, generated JavaScript, `<html data-theme>`, localStorage, and the header theme button.
- Each theme is one flat JSON singleton containing the complete variable set documented in `static/themes/README.md`.
- `build.rs` validates the singleton JSONs at compile time and fails the build if a theme omits a required variable.

Run locally:

```bash
cargo run
```

Build release:

```bash
cargo build --release
```

Manual live bridge while the update manager is being crafted:

```bash
rsync -a --delete ./ root@192.168.123.54:/opt/arcadia-src/
ssh root@192.168.123.54 'cd /opt/arcadia-src && cargo build --release'
ssh root@192.168.123.54 'install -m 0755 /opt/arcadia-src/target/release/arcadia /usr/local/bin/arcadia'
```

Runtime service target:

```text
ExecStart=/usr/local/bin/arcadia
Environment=ARCADIA_BIND=0.0.0.0:8080
```

Public-safe boundary: do not commit PINs, Keyman material, ROMs, BIOS files, save data, private receipts, provider credentials, or household secrets.
