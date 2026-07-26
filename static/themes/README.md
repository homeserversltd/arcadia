# Arcadia theme JSON doctrine

Arcadia themes live at this exact repo path:

```text
static/themes/*.json
```

Inside a Cibation worktree the same authoritative theme files live at:

```text
static/themes/*.json (in any checkout or worktree)
```

The filename stem is the theme name. `static/themes/crown-noir.json` becomes the propagated theme name `crown-noir` in generated CSS, generated JavaScript, the `<html data-theme="...">` state, localStorage, and the Arcadia header theme button.

Each theme is one flat JSON object. Every key below is required and every value is a CSS value string:

```text
bg-base
bg-shell
bg-panel
bg-panel-strong
bg-raised
bg-overlay
fg-primary
fg-muted
fg-inverse
accent
accent-strong
accent-soft
accent-contrast
line
line-strong
glass
glass-strong
shadow
shadow-strong
good
warn
bad
idle
focus
danger
danger-strong
storage-games
storage-artwork
storage-ai
storage-other
storage-free
storage-updates
storage-logs
storage-temporary
storage-system
modal-backdrop
button-surface
button-surface-strong
field-surface
code-accent
hero-good-surface
hero-good-line
warning-surface
warning-line
toast-border
radius-sm
radius-md
radius-lg
radius-xl
space-xs
space-sm
space-md
space-lg
font-family
```

Build authority:

- `build.rs` validates every theme JSON at compile time.
- Missing keys, empty strings, invalid JSON, or non-object JSON fail the build.
- Generated CSS variables are emitted from the JSON singleton into `themes.css`.
- Generated JavaScript emits `window.ARCADIA_THEMES`, using each filename stem as the canonical name.
- `static/app.css` consumes the generated variables instead of owning theme color values directly.

To add a theme, add one complete JSON file under `static/themes/`, run `cargo test`, and use the header theme button to cycle to the new filename-derived theme name.
