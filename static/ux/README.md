# Arcadia UX library

This directory is the canonical agent-facing UX substrate for Arcadia. Arcadia UI work starts here: reusable layout, component scale, control/action tracks, viewport policy, and ordinary text bounds belong in this library before feature selectors receive domain-specific composition.

Files:

- `arcadia-ux.css` owns shell rhythm, component scale, control/action tracks, spacing, readable text bounds, and utility primitives.
- `arcadia-viewports.css` owns all responsive viewport dialing. Desktop is the unqualified baseline; tablet/phone changes live in the two media bands here.
- `../themes/*.json` owns skin values only: color, radius, spacing constants, and theme surfaces. Theme JSON does not own layout breakpoints or per-viewport selector surgery.

Agent contract:

1. Add or adjust shared UX variables in `arcadia-ux.css`.
2. Put tablet/phone layout changes in `arcadia-viewports.css`.
3. Keep `static/app/**` composable modules for view/component composition that consumes UX/theme variables and is not viewport-specific. `static/app.css` is a hoist only; `build.rs` composes the spine into `APP_CSS`.
4. Do not add new `@media` blocks to `static/app.css`; the Rust test suite enforces this.
5. Keep ordinary human-facing text within 12px through 22px unless a special display surface is explicitly ordered.
6. Prove visible control geometry with rendered DOM readback when a change touches buttons, cards, action rows, or viewport fit.
7. Pane-blind living-state style binding uses `data-bind-style-var="--var:path"`; values come from curated API document strings and are written only to CSS custom properties.
