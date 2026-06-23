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

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ControllerTupleRecord {
    control: String,
    input: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ControllerRecord {
    id: String,
    name: String,
    handler: String,
    path: String,
    glyph: String,
    transport: String,
    kind: String,
    layout_style: String,
    first_seen: String,
    last_seen: String,
    tuples: Vec<ControllerTupleRecord>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ControllerLibrary {
    schema: String,
    active_id: String,
    controllers: BTreeMap<String, ControllerRecord>,
}

fn controller_library_path() -> PathBuf {
    controller_profile_root().join("library.json")
}

fn empty_controller_library() -> ControllerLibrary {
    ControllerLibrary {
        schema: "arcadia.controller_library.v1".to_string(),
        active_id: String::new(),
        controllers: BTreeMap::new(),
    }
}

fn load_controller_library() -> ControllerLibrary {
    let path = controller_library_path();
    let Ok(text) = fs::read_to_string(&path) else {
        let mut library = empty_controller_library();
        migrate_legacy_default_profile(&mut library);
        return library;
    };
    let Ok(mut library) = serde_json::from_str::<ControllerLibrary>(&text) else {
        let mut library = empty_controller_library();
        migrate_legacy_default_profile(&mut library);
        return library;
    };
    if library.schema != "arcadia.controller_library.v1" {
        library.schema = "arcadia.controller_library.v1".to_string();
    }
    if library.controllers.is_empty() {
        migrate_legacy_default_profile(&mut library);
    }
    library
}

fn save_controller_library(library: &ControllerLibrary) -> std::io::Result<()> {
    let path = controller_library_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_string_pretty(library).map_err(|error| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string())
    })?;
    fs::write(path, format!("{body}\n"))
}

fn bindings_from_tuples(tuples: &[ControllerTupleRecord]) -> Vec<ControllerBindingStatus> {
    if tuples.is_empty() {
        return default_controller_bindings();
    }
    let mut bindings = Vec::with_capacity(tuples.len());
    for tuple in tuples {
        bindings.push(ControllerBindingStatus {
            control: canonical_control_name(&tuple.control),
            binding: tuple.input.clone(),
            pressed: false,
        });
    }
    if bindings.is_empty() {
        default_controller_bindings()
    } else {
        bindings
    }
}

fn tuples_from_bindings(bindings: &[ControllerBindingStatus]) -> Vec<ControllerTupleRecord> {
    bindings
        .iter()
        .map(|binding| ControllerTupleRecord {
            control: canonical_control_name(&binding.control),
            input: binding.binding.clone(),
        })
        .collect()
}

fn controller_id_for_device(device: &ControllerDeviceStatus) -> String {
    let base = safe_file_stem(&device.name);
    if base.is_empty() {
        return safe_file_stem(&device.handler);
    }
    let library = load_controller_library();
    if library.controllers.contains_key(&base) {
        return base;
    }
    for (id, record) in &library.controllers {
        if record.name == device.name {
            return id.clone();
        }
    }
    let suffix = safe_file_stem(&device.path);
    if suffix.is_empty() || suffix == base {
        base
    } else {
        format!("{base}-{suffix}")
    }
}

fn migrate_legacy_default_profile(library: &mut ControllerLibrary) {
    if !library.controllers.is_empty() {
        return;
    }
    let path = controller_profile_path();
    let Ok(text) = fs::read_to_string(&path) else {
        return;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return;
    };
    let device_name = value
        .get("device")
        .and_then(|v| v.as_str())
        .unwrap_or("Saved controller");
    let handler = value
        .get("handler")
        .and_then(|v| v.as_str())
        .unwrap_or("virtual0");
    let items = value
        .get("tuples")
        .and_then(|v| v.as_array())
        .or_else(|| value.get("bindings").and_then(|v| v.as_array()));
    let Some(items) = items else {
        return;
    };
    let mut tuples = Vec::new();
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
        tuples.push(ControllerTupleRecord {
            control: canonical_control_name(control),
            input: input.to_string(),
        });
    }
    if tuples.is_empty() {
        return;
    }
    let id = safe_file_stem(device_name);
    let now = human_now_label();
    library.controllers.insert(
        id.clone(),
        ControllerRecord {
            id: id.clone(),
            name: device_name.to_string(),
            handler: handler.to_string(),
            path: format!("/dev/input/{handler}"),
            glyph: controller_glyph_from_name(device_name),
            transport: controller_transport_from_name(device_name, handler),
            kind: "gamepad".to_string(),
            layout_style: "Default".to_string(),
            first_seen: now.clone(),
            last_seen: now,
            tuples,
        },
    );
    if library.active_id.is_empty() {
        library.active_id = id;
    }
    let _ = save_controller_library(library);
}

fn remember_controller_devices(devices: &[ControllerDeviceStatus]) -> ControllerLibrary {
    let mut library = load_controller_library();
    let now = human_now_label();
    for device in devices {
        let id = controller_id_for_device(device);
        let entry = library
            .controllers
            .entry(id.clone())
            .or_insert_with(|| ControllerRecord {
                id: id.clone(),
                name: device.name.clone(),
                handler: device.handler.clone(),
                path: device.path.clone(),
                glyph: device.glyph.clone(),
                transport: device.transport.clone(),
                kind: device.kind.clone(),
                layout_style: "Default".to_string(),
                first_seen: now.clone(),
                last_seen: now.clone(),
                tuples: tuples_from_bindings(&default_controller_bindings()),
            });
        entry.name = device.name.clone();
        entry.handler = device.handler.clone();
        entry.path = device.path.clone();
        entry.glyph = device.glyph.clone();
        entry.transport = device.transport.clone();
        entry.kind = device.kind.clone();
        entry.last_seen = now.clone();
    }
    if library.active_id.is_empty() {
        if let Some(device) = devices.first() {
            library.active_id = controller_id_for_device(device);
        } else if let Some(id) = library.controllers.keys().next() {
            library.active_id = id.clone();
        }
    } else if !library.controllers.contains_key(&library.active_id) {
        if let Some(device) = devices.first() {
            library.active_id = controller_id_for_device(device);
        } else if let Some(id) = library.controllers.keys().next() {
            library.active_id = id.clone();
        } else {
            library.active_id.clear();
        }
    }
    let _ = save_controller_library(&library);
    library
}

fn active_controller_id() -> String {
    load_controller_library().active_id
}

fn set_active_controller_id(controller_id: &str) -> std::io::Result<()> {
    let mut library = load_controller_library();
    if !library.controllers.contains_key(controller_id) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "controller not in library",
        ));
    }
    library.active_id = controller_id.to_string();
    save_controller_library(&library)
}

fn bindings_for_controller_id(controller_id: &str) -> Vec<ControllerBindingStatus> {
    let library = load_controller_library();
    library
        .controllers
        .get(controller_id)
        .map(|record| bindings_from_tuples(&record.tuples))
        .unwrap_or_else(default_controller_bindings)
}

fn bindings_for_active_controller() -> Vec<ControllerBindingStatus> {
    let library = load_controller_library();
    if library.active_id.is_empty() {
        return default_controller_bindings();
    }
    bindings_for_controller_id(&library.active_id)
}

fn save_bindings_for_controller(
    controller_id: &str,
    bindings: &[ControllerBindingStatus],
    layout_style: Option<&str>,
) -> std::io::Result<()> {
    let mut library = load_controller_library();
    let Some(record) = library.controllers.get_mut(controller_id) else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "controller not in library",
        ));
    };
    record.tuples = tuples_from_bindings(bindings);
    record.last_seen = human_now_label();
    if let Some(style) = layout_style.filter(|value| !value.trim().is_empty()) {
        record.layout_style = style.to_string();
    }
    let device_name = record.name.clone();
    let handler = record.handler.clone();
    save_controller_library(&library)?;
    let _ = write_controller_profile(
        &controller_profile_path(),
        &device_name,
        &handler,
        bindings,
    );
    Ok(())
}

fn active_controller_device_tuple() -> (String, String, String, Vec<ControllerBindingStatus>) {
    let library = load_controller_library();
    if let Some(record) = library.controllers.get(&library.active_id) {
        return (
            record.id.clone(),
            record.name.clone(),
            record.handler.clone(),
            bindings_from_tuples(&record.tuples),
        );
    }
    (
        String::new(),
        "No controller selected".to_string(),
        "virtual-arcadia-gamepad".to_string(),
        default_controller_bindings(),
    )
}

fn controller_pool_entries(devices: &[ControllerDeviceStatus]) -> Vec<ControllerPoolEntry> {
    let library = remember_controller_devices(devices);
    let connected_ids: BTreeSet<String> = devices.iter().map(controller_id_for_device).collect();
    let mut entries: Vec<ControllerPoolEntry> = library
        .controllers
        .values()
        .map(|record| {
            let bindings = bindings_from_tuples(&record.tuples);
            ControllerPoolEntry {
                id: record.id.clone(),
                name: record.name.clone(),
                handler: record.handler.clone(),
                path: record.path.clone(),
                glyph: record.glyph.clone(),
                transport: record.transport.clone(),
                kind: record.kind.clone(),
                state: if connected_ids.contains(&record.id) {
                    "connected".to_string()
                } else {
                    "remembered".to_string()
                },
                layout_style: record.layout_style.clone(),
                tuple_count: record.tuples.len(),
                bindings,
                last_seen: record.last_seen.clone(),
                selected: record.id == library.active_id,
            }
        })
        .collect();
    entries.sort_by(|a, b| {
        b.selected
            .cmp(&a.selected)
            .then_with(|| b.state.cmp(&a.state))
            .then_with(|| b.last_seen.cmp(&a.last_seen))
            .then_with(|| a.name.cmp(&b.name))
    });
    entries
}

fn saved_or_default_controller_bindings() -> Vec<ControllerBindingStatus> {
    bindings_for_active_controller()
}

fn controller_glyph_from_name(name: &str) -> String {
    let lowered = name.to_ascii_lowercase();
    if lowered.contains("8bitdo") {
        "8B".to_string()
    } else if lowered.contains("xbox") {
        "XB".to_string()
    } else if lowered.contains("dual") || lowered.contains("playstation") {
        "PS".to_string()
    } else if lowered.contains("nintendo") || lowered.contains("switch") {
        "NS".to_string()
    } else {
        "GP".to_string()
    }
}

fn controller_transport_from_name(name: &str, path: &str) -> String {
    let lowered = format!("{name} {path}").to_ascii_lowercase();
    if lowered.contains("bluetooth") {
        "Bluetooth".to_string()
    } else if lowered.contains("usb") {
        "USB".to_string()
    } else if lowered.contains("8bitdo") {
        "2.4G / USB".to_string()
    } else {
        "Input".to_string()
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