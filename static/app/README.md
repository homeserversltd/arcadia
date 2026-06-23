# Arcadia app CSS spine

`static/app.css` is a thin hoist. The served application stylesheet band is composed at build time from this tree.

## Law

- **Tokens and theme aliases** live in `base/`.
- **Shell chrome** (header, sidebar, workspace) lives in `shell/`.
- **Reusable composables** (buttons, modals, pin gate) live in `components/`.

### Fullscreen modal variant

Use `PopupManager.showModal({ variant: 'fullscreen', ... })` for edge-to-edge viewport takeovers (controller map, future workbench modals). Tokens: `--ux-modal-fullscreen-padding`, `--ux-modal-fullscreen-gap`. Pane-specific layout hooks target `.modal-card--fullscreen .<pane-block>` in `views/`.

### Loading ring (`ArcadiaLoading`)

Brand spinner lives in `components/loading.css` (`ux-arcadia-spinner`). Use everywhere async work blocks UI:

```javascript
ArcadiaLoading.spinner({ label: 'Working…', size: 'md' });
ArcadiaLoading.showIn(hostElement, { label: 'Loading' });
ArcadiaLoading.showOverlay({ label: 'Saving' });
await ArcadiaLoading.during(fetchWork(), { target: host, label: 'Loading' });
```
- **Viewport pane composition** lives in `views/` — one file per major pane where practical.
- **Desktop single-pane fit overrides** live in `desktop-fit/` — not `@media` bands (those stay in `ux/arcadia-viewports.css`).

## Spine order

Read `index.json` at each band. `build.rs` walks `children` in array order and concatenates leaf `.css` files into `OUT_DIR/app-composed.css`, which becomes `APP_CSS` in `src/bands/constants.rs`.

When adding a new composable, create a discrete `.css` file under the owning band, register it in that band's `index.json`, and keep each file near or under ~500 lines.