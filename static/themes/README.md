# Arcadia theme files

Arcadia themes are JSON files in `static/themes/`.

The filename stem is the theme name. For example, `static/themes/crown-noir.json` produces the selectable theme name `crown-noir` in generated CSS and JavaScript, the `<html data-theme="...">` state, browser storage, and the console header control.

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

## Build behavior

- `build.rs` validates every theme JSON file at compile time.
- Missing keys, empty strings, invalid JSON, and non-object JSON fail the build.
- The build emits CSS variables into `themes.css`.
- The build emits `window.ARCADIA_THEMES`, using each filename stem as its canonical name.
- `static/app.css` consumes generated variables instead of declaring theme color values directly.

To add a theme, add one complete JSON file under `static/themes/`, run `cargo test`, and select the filename-derived theme name from the console header.
