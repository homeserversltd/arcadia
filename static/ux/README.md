# Arcadia UX library

This directory is the canonical agent-facing UX substrate for Arcadia. Agents should add reusable layout, component, and viewport policy here before touching scattered feature selectors.

Files:

- `arcadia-ux.css` owns shell rhythm, component scale, spacing, readable text bounds, and utility primitives.
- `arcadia-viewports.css` owns all responsive viewport dialing. Desktop is the unqualified baseline; tablet/phone changes live in the two media bands here.
- `../themes/*.json` owns skin values only: color, radius, spacing constants, and theme surfaces. Theme JSON does not own layout breakpoints or per-viewport selector surgery.

Agent contract:

1. Add or adjust shared UX variables in `arcadia-ux.css`.
2. Put tablet/phone layout changes in `arcadia-viewports.css`.
3. Keep `static/app.css` for view/component defaults that are not viewport-specific.
4. Do not add new `@media` blocks to `static/app.css`; the Rust test suite enforces this.
5. Keep ordinary human-facing text within 12px through 22px unless a special display surface is explicitly ordered.
