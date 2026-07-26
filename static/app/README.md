# Arcadia application CSS

`static/app.css` is a thin include spine. The served application stylesheet is assembled at build time from this tree.

## Organization

- **Tokens and theme aliases** live in `base/`.
- **Shell chrome** for the header, sidebar, and workspace lives in `shell/`.
- **Reusable components** such as buttons, modals, and the PIN gate live in `components/`.
- **View composition** lives in `views/`, usually one file per major console view.
- **Desktop fit overrides** live in `desktop-fit/`; viewport media rules remain in `static/ux/arcadia-viewports.css`.

## Fullscreen modal variant

Use `PopupManager.showModal({ variant: 'fullscreen', ... })` for edge-to-edge view takeovers such as the controller map. The relevant tokens are `--ux-modal-fullscreen-padding` and `--ux-modal-fullscreen-gap`; pane-specific hooks belong in `views/` under `.modal-card--fullscreen .<pane-block>`.

## Loading ring

The application loading ring lives in `components/loading.css` as `ux-arcadia-spinner`. Use it for visible asynchronous work:

```javascript
ArcadiaLoading.spinner({ label: 'Working…', size: 'md' });
ArcadiaLoading.showIn(hostElement, { label: 'Loading' });
ArcadiaLoading.showOverlay({ label: 'Saving' });
await ArcadiaLoading.during(fetchWork(), { target: host, label: 'Loading' });
```

## Build order

Read `index.json` at each band. `build.rs` walks `children` in array order and concatenates leaf `.css` files into `OUT_DIR/app-composed.css`, then exposes it as `APP_CSS` in `src/bands/constants.rs`.

When adding a reusable component, create a focused `.css` file in its owning band, register it in that band's `index.json`, and keep each file reasonably small.
