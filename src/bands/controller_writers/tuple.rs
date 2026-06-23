const KNOWN_EMULATORS: &[&str] = &["RetroArch", "Dolphin", "DuckStation", "PCSX2", "PPSSPP"];

fn default_controller_bindings() -> Vec<ControllerBindingStatus> {
    [
        ("A", "button 0"),
        ("B", "button 1"),
        ("X", "button 2"),
        ("Y", "button 3"),
        ("L1", "button 4"),
        ("R1", "button 5"),
        ("L2", "axis 2"),
        ("R2", "axis 5"),
        ("L3", "button 8"),
        ("R3", "button 9"),
        ("Select", "button 6"),
        ("Start", "button 7"),
        ("D-pad Up", "hat 0 up"),
        ("D-pad Down", "hat 0 down"),
        ("D-pad Left", "hat 0 left"),
        ("D-pad Right", "hat 0 right"),
        ("Left Stick X", "axis 0"),
        ("Left Stick Y", "axis 1"),
        ("Right Stick X", "axis 3"),
        ("Right Stick Y", "axis 4"),
    ]
    .into_iter()
    .map(|(control, binding)| ControllerBindingStatus {
        control: control.to_string(),
        binding: binding.to_string(),
        pressed: false,
    })
    .collect()
}

fn controller_bindings_for_profile(profile: &str) -> Vec<ControllerBindingStatus> {
    let mut bindings = default_controller_bindings();
    match profile.trim().to_ascii_lowercase().as_str() {
        "nintendo" => {
            upsert_binding(&mut bindings, "A", "button 1");
            upsert_binding(&mut bindings, "B", "button 0");
            upsert_binding(&mut bindings, "X", "button 3");
            upsert_binding(&mut bindings, "Y", "button 2");
        }
        "playstation" => {
            upsert_binding(&mut bindings, "A", "button 1");
            upsert_binding(&mut bindings, "B", "button 2");
            upsert_binding(&mut bindings, "X", "button 0");
            upsert_binding(&mut bindings, "Y", "button 3");
        }
        "arcade" => {
            upsert_binding(&mut bindings, "A", "button 0");
            upsert_binding(&mut bindings, "B", "button 1");
            upsert_binding(&mut bindings, "X", "button 4");
            upsert_binding(&mut bindings, "Y", "button 5");
            upsert_binding(&mut bindings, "L1", "button 2");
            upsert_binding(&mut bindings, "R1", "button 3");
        }
        _ => {}
    }
    bindings
}

fn saved_or_default_controller_bindings() -> Vec<ControllerBindingStatus> {
    let path = controller_profile_path();
    let Ok(text) = fs::read_to_string(&path) else {
        return default_controller_bindings();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return default_controller_bindings();
    };
    let items = value
        .get("tuples")
        .and_then(|v| v.as_array())
        .or_else(|| value.get("bindings").and_then(|v| v.as_array()));
    let Some(items) = items else {
        return default_controller_bindings();
    };
    if items.is_empty() {
        return default_controller_bindings();
    }
    let mut bindings = Vec::with_capacity(items.len());
    for item in items {
        let Some(control) = item.get("control").and_then(|v| v.as_str()) else {
            continue;
        };
        let input = item
            .get("input")
            .and_then(|v| v.as_str())
            .or_else(|| item.get("binding").and_then(|v| v.as_str()));
        let Some(input) = input else {
            continue;
        };
        bindings.push(ControllerBindingStatus {
            control: canonical_control_name(control),
            binding: input.to_string(),
            pressed: false,
        });
    }
    if bindings.is_empty() {
        default_controller_bindings()
    } else {
        bindings
    }
}

fn upsert_binding(bindings: &mut Vec<ControllerBindingStatus>, control: &str, input: &str) {
    let canonical = canonical_control_name(control);
    if let Some(existing) = bindings
        .iter_mut()
        .find(|b| canonical_control_name(&b.control) == canonical)
    {
        existing.control = canonical;
        existing.binding = input.to_string();
        existing.pressed = false;
    } else {
        bindings.push(ControllerBindingStatus {
            control: canonical,
            binding: input.to_string(),
            pressed: false,
        });
    }
}

fn canonical_control_name(control: &str) -> String {
    match control.trim().to_ascii_lowercase().as_str() {
        "left stick" | "left stick x" | "leftstick" | "leftstickx" => "Left Stick X".to_string(),
        "left stick y" | "leftsticky" => "Left Stick Y".to_string(),
        "right stick" | "right stick x" | "rightstick" | "rightstickx" => {
            "Right Stick X".to_string()
        }
        "right stick y" | "rightsticky" => "Right Stick Y".to_string(),
        "d-pad up" | "dpad up" | "up" => "D-pad Up".to_string(),
        "d-pad down" | "dpad down" | "down" => "D-pad Down".to_string(),
        "d-pad left" | "dpad left" | "left" => "D-pad Left".to_string(),
        "d-pad right" | "dpad right" | "right" => "D-pad Right".to_string(),
        "l3" => "L3".to_string(),
        "r3" => "R3".to_string(),
        other => other
            .split_whitespace()
            .map(|part| {
                if part.eq_ignore_ascii_case("d-pad") {
                    "D-pad".to_string()
                } else if part.len() <= 2 {
                    part.to_ascii_uppercase()
                } else {
                    let mut chars = part.chars();
                    match chars.next() {
                        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                        None => String::new(),
                    }
                }
            })
            .collect::<Vec<_>>()
            .join(" "),
    }
}

fn binding_for_control(control: &str) -> String {
    let canonical = canonical_control_name(control);
    default_controller_bindings()
        .into_iter()
        .find(|b| b.control == canonical)
        .map(|b| b.binding)
        .unwrap_or_else(|| format!("virtual:{}", safe_file_stem(&canonical)))
}

fn write_controller_profile(
    path: &Path,
    device_name: &str,
    handler: &str,
    bindings: &[ControllerBindingStatus],
) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let body = controller_profile_json(device_name, handler, bindings);
    fs::write(path, body)
}

fn controller_profile_json(
    device_name: &str,
    handler: &str,
    bindings: &[ControllerBindingStatus],
) -> String {
    let tuples_json = bindings
        .iter()
        .map(|b| {
            format!(
                "    {{\"control\":\"{}\",\"input\":\"{}\"}}",
                json_escape(&canonical_control_name(&b.control)),
                json_escape(&b.binding)
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    format!(
        "{{\n  \"schema\": \"arcadia.controller_profile.v1\",\n  \"name\": \"Default\",\n  \"device\": \"{}\",\n  \"handler\": \"{}\",\n  \"tuples\": [\n{}\n  ]\n}}\n",
        json_escape(device_name),
        json_escape(handler),
        tuples_json
    )
}

fn controller_profile_root() -> PathBuf {
    if let Ok(root) = env::var("ARCADIA_CONTROLLER_PROFILE_ROOT") {
        return PathBuf::from(root);
    }
    let primary = PathBuf::from("/var/lib/arcadia/controller-profiles");
    if primary.exists()
        || primary
            .parent()
            .map(|p| p.exists() && is_writable_dir(p))
            .unwrap_or(false)
    {
        return primary;
    }
    PathBuf::from("/tmp/arcadia/controller-profiles")
}

fn controller_profile_path() -> PathBuf {
    controller_profile_root().join("default.json")
}

fn binding_number(binding: &str) -> String {
    binding.split_whitespace().last().unwrap_or(binding).to_string()
}

fn json_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn safe_file_stem(value: &str) -> String {
    value
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn is_writable_dir(path: &Path) -> bool {
    let probe = path.join(format!(".arcadia-write-probe-{}", std::process::id()));
    match fs::write(&probe, b"probe") {
        Ok(()) => {
            let _ = fs::remove_file(probe);
            true
        }
        Err(_) => false,
    }
}