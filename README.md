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
