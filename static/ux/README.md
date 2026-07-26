# Arcadia UX library

This directory contains Arcadia's shared UX library: reusable layout, component scale, control tracks, responsive policy, and ordinary text bounds. Build shared behavior here before adding feature-specific selectors.

## Files

- `arcadia-ux.css` owns shell rhythm, component scale, control tracks, spacing, readable text bounds, and utility primitives.
- `arcadia-viewports.css` owns responsive layout. Desktop is the baseline; tablet and phone changes live in its media bands.
- `../themes/*.json` owns visual tokens such as color, radius, spacing, and surface values. Theme files do not own layout breakpoints or per-viewport selector changes.

## Contribution contract

1. Add or adjust shared UX variables in `arcadia-ux.css`.
2. Put tablet and phone layout changes in `arcadia-viewports.css`.
3. Keep `static/app/**` for view and component composition that consumes UX or theme variables and is not viewport-specific. `static/app.css` is an include spine; `build.rs` composes it into `APP_CSS`.
4. Do not add new `@media` blocks to `static/app.css`; the Rust test suite enforces this boundary.
5. Keep ordinary user-facing text within 12px through 22px unless a display surface explicitly needs a different scale.
6. Verify rendered control geometry when a change affects buttons, cards, action rows, or viewport fit.
7. Pane-independent living-state style binding uses `data-bind-style-var="--var:path"`; curated API document strings are written only to CSS custom properties.
8. High-bandwidth pane widgets register with `ArcadiaProjector.registerWidget(name, fn)`. A widget receives the living-state document and may mount a scoped transport only while its visible presenter is active. The `controllersPane` widget combines controller readbacks with `/api/controllers/trainer/events` for trainer lighting and closes the stream when its pane or modal loses focus.
