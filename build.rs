use std::{env, fs, path::PathBuf};

const REQUIRED: &[&str] = &[
    "bg-base",
    "bg-shell",
    "bg-panel",
    "bg-panel-strong",
    "bg-raised",
    "bg-overlay",
    "fg-primary",
    "fg-muted",
    "fg-inverse",
    "accent",
    "accent-strong",
    "accent-soft",
    "accent-contrast",
    "line",
    "line-strong",
    "glass",
    "glass-strong",
    "shadow",
    "shadow-strong",
    "good",
    "warn",
    "bad",
    "idle",
    "focus",
    "danger",
    "danger-strong",
    "storage-games",
    "storage-artwork",
    "storage-ai",
    "storage-other",
    "storage-free",
    "storage-updates",
    "storage-logs",
    "storage-temporary",
    "storage-system",
    "modal-backdrop",
    "button-surface",
    "button-surface-strong",
    "field-surface",
    "code-accent",
    "hero-good-surface",
    "hero-good-line",
    "warning-surface",
    "warning-line",
    "toast-border",
    "radius-sm",
    "radius-md",
    "radius-lg",
    "radius-xl",
    "space-xs",
    "space-sm",
    "space-md",
    "space-lg",
    "font-family",
];

fn main() {
    println!("cargo:rerun-if-changed=static/themes");
    let dir = PathBuf::from("static/themes");
    let mut files = fs::read_dir(&dir)
        .expect("static/themes exists")
        .map(|entry| entry.expect("theme dir entry").path())
        .filter(|path| path.extension().and_then(|s| s.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    files.sort();
    if files.is_empty() {
        panic!("Arcadia needs at least one JSON theme");
    }
    let mut css = String::from(
        "/* generated from static/themes/*.json; filename stem is the theme name */\n",
    );
    let mut manifest = String::from("window.ARCADIA_THEMES = [\n");
    for path in files {
        println!("cargo:rerun-if-changed={}", path.display());
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("theme name");
        let raw = fs::read_to_string(&path).expect("theme json readable");
        let value: serde_json::Value = serde_json::from_str(&raw).expect("theme json parses");
        let obj = value.as_object().expect("theme json is a flat object");
        for key in REQUIRED {
            let v = obj
                .get(*key)
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| panic!("theme {name} missing string key {key}"));
            if v.trim().is_empty() {
                panic!("theme {name} key {key} is empty");
            }
        }
        css.push_str(&format!(
            ":root[data-theme=\"{}\"] {{\n  --theme-name: \"{}\";\n",
            name, name
        ));
        for key in REQUIRED {
            let v = obj.get(*key).and_then(|v| v.as_str()).unwrap();
            css.push_str(&format!("  --theme-{}: {};\n", key, v));
        }
        css.push_str("}\n");
        manifest.push_str(&format!(
            "  {{ name: {:?}, label: {:?} }},\n",
            name,
            label(name)
        ));
    }
    manifest.push_str("];\n");
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    fs::write(out.join("themes.css"), css).expect("write generated themes.css");
    fs::write(out.join("themes.js"), manifest).expect("write generated themes.js");
}

fn label(name: &str) -> String {
    name.split('-')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
