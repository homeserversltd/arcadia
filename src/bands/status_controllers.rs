fn controller_status() -> ControllerStatus {
    let devices = controller_devices();
    let detected_count = devices.len();
    let recovery = controller_recovery_status(&devices);
    let state = if detected_count > 0 { "connected".to_string() } else { recovery.state.clone() };
    let primary_device = devices
        .first()
        .map(|device| device.name.clone())
        .unwrap_or_else(|| recovery.title.clone());
    let profile = controller_profile_status(devices.first());
    let live_input = read_controller_input(devices.first());
    ControllerStatus {
        state,
        detected_count,
        primary_device,
        last_scan: human_now_label(),
        recovery,
        devices,
        profile,
        profile_presets: controller_profile_presets(),
        live_input,
        emulators: emulator_controller_statuses(),
    }
}

fn controller_devices() -> Vec<ControllerDeviceStatus> {
    let mut devices = controllers_from_proc_bus_input();
    if devices.is_empty() {
        devices = controllers_from_dev_input();
    }
    devices.sort_by(|a, b| controller_rank(a).cmp(&controller_rank(b)).then(a.name.cmp(&b.name)));
    devices.dedup_by(|a, b| a.path == b.path || a.handler == b.handler);
    devices
}

fn controller_recovery_status(devices: &[ControllerDeviceStatus]) -> ControllerRecoveryStatus {
    if let Some(device) = devices.first() {
        return ControllerRecoveryStatus {
            state: "connected".to_string(),
            title: format!("{} ready", device.name),
            detail: "Press a button or move a stick; the controller face lights live.".to_string(),
            action: "Map or assign profile".to_string(),
        };
    }
    if let Some(receiver) = idle_receiver_label() {
        return ControllerRecoveryStatus {
            state: "receiver-only".to_string(),
            title: receiver,
            detail: "Receiver is awake; no gamepad event surface is exposed yet.".to_string(),
            action: "Wake or pair the controller".to_string(),
        };
    }
    ControllerRecoveryStatus {
        state: "disconnected".to_string(),
        title: "No controller connected".to_string(),
        detail: "Plug in USB, wake Bluetooth, or pair the 2.4G receiver.".to_string(),
        action: "Scan controllers".to_string(),
    }
}

fn idle_receiver_label() -> Option<String> {
    for root in ["/dev/input/by-id", "/dev/hidraw0"] {
        if root == "/dev/hidraw0" && Path::new(root).exists() {
            return Some("Receiver only".to_string());
        }
        let Ok(entries) = fs::read_dir(root) else { continue; };
        for entry in entries.flatten() {
            let raw = entry.file_name().to_string_lossy().to_string();
            let lowered = raw.to_ascii_lowercase();
            if lowered.contains("8bitdo") && (lowered.contains("idle") || lowered.contains("hidraw")) {
                return Some(controller_display_name(&raw));
            }
        }
    }
    None
}

fn controller_transport(name: &str, path: &str) -> String {
    let lowered = format!("{} {}", name, path).to_ascii_lowercase();
    if lowered.contains("bluetooth") { "Bluetooth" }
    else if lowered.contains("usb") { "USB" }
    else if lowered.contains("8bitdo") { "2.4G / USB" }
    else { "Input" }.to_string()
}

fn controller_glyph(name: &str) -> String {
    let lowered = name.to_ascii_lowercase();
    if lowered.contains("8bitdo") { "8B" }
    else if lowered.contains("xbox") { "XB" }
    else if lowered.contains("dual") || lowered.contains("playstation") { "PS" }
    else if lowered.contains("nintendo") || lowered.contains("switch") { "NS" }
    else { "GP" }.to_string()
}

fn controller_profile_presets() -> Vec<ControllerProfilePresetStatus> {
    [("Default", "Xbox / SDL order", "active"), ("Nintendo", "A/B swapped", "available"), ("PlayStation", "Cross/Circle labels", "available"), ("Arcade", "D-pad priority", "available")]
        .into_iter()
        .map(|(name, layout, state)| ControllerProfilePresetStatus { name: name.to_string(), layout: layout.to_string(), state: state.to_string() })
        .collect()
}

fn controller_rank(device: &ControllerDeviceStatus) -> u8 {
    if device.handler.starts_with("js") { 0 } else if device.kind == "gamepad" { 1 } else { 2 }
}

fn controllers_from_proc_bus_input() -> Vec<ControllerDeviceStatus> {
    let Ok(text) = fs::read_to_string("/proc/bus/input/devices") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for block in text.split("\n\n") {
        let name = block
            .lines()
            .find_map(|line| line.strip_prefix("N: Name="))
            .map(|value| value.trim_matches('"').to_string())
            .unwrap_or_else(|| "Controller".to_string());
        let handlers = block
            .lines()
            .find_map(|line| line.strip_prefix("H: Handlers="))
            .unwrap_or("");
        if !is_real_controller_block(&name, handlers, block) {
            continue;
        }
        let handler = handlers
            .split_whitespace()
            .find(|part| part.starts_with("js"))
            .or_else(|| handlers.split_whitespace().find(|part| part.starts_with("event")))
            .unwrap_or("event?");
        let path = if handler.starts_with("js") || handler.starts_with("event") {
            format!("/dev/input/{}", handler)
        } else {
            "/dev/input".to_string()
        };
        out.push(ControllerDeviceStatus {
            name: name.clone(),
            handler: handler.to_string(),
            kind: if handler.starts_with("js") { "joystick" } else { "gamepad" }.to_string(),
            path: path.clone(),
            state: "detected".to_string(),
            transport: controller_transport(&name, &path),
            glyph: controller_glyph(&name),
        });
    }
    out
}

fn is_real_controller_block(name: &str, handlers: &str, block: &str) -> bool {
    let lowered = name.to_ascii_lowercase();
    let rejected = ["keyboard", "mouse", "led", "hid events", "button array", "rfkill", "power button", "video bus"];
    if rejected.iter().any(|needle| lowered.contains(needle)) {
        return false;
    }
    let has_js = handlers.split_whitespace().any(|part| part.starts_with("js"));
    if has_js {
        return true;
    }
    let has_event = handlers.split_whitespace().any(|part| part.starts_with("event"));
    let has_axes = block.lines().any(|line| line.starts_with("B: ABS=") && !line.ends_with('0'));
    let controllerish = ["gamepad", "controller", "xbox", "playstation", "dualshock", "dualsense", "8bitdo", "nintendo", "switch", "joystick"]
        .iter()
        .any(|needle| lowered.contains(needle));
    has_event && has_axes && controllerish
}

fn controllers_from_dev_input() -> Vec<ControllerDeviceStatus> {
    let mut out = Vec::new();
    for root in ["/dev/input/by-id", "/dev/input"] {
        let Ok(entries) = fs::read_dir(root) else { continue; };
        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();
            let lowered = file_name.to_ascii_lowercase();
            if !is_real_controller_name(&lowered) {
                continue;
            }
            if lowered.contains("-event-kbd") || lowered.contains("-event-mouse") || lowered.contains("hidraw") || lowered.contains("led") {
                continue;
            }
            out.push(ControllerDeviceStatus {
                name: controller_display_name(&file_name),
                handler: file_name.clone(),
                kind: if lowered.starts_with("js") || lowered.ends_with("-joystick") { "joystick" } else { "gamepad" }.to_string(),
                path: path.display().to_string(),
                state: "detected".to_string(),
                transport: controller_transport(&file_name, &path.display().to_string()),
                glyph: controller_glyph(&file_name),
            });
        }
    }
    out
}

fn is_real_controller_name(lowered: &str) -> bool {
    lowered.starts_with("js")
        || lowered.contains("event-joystick")
        || lowered.ends_with("-joystick")
        || lowered.contains("gamepad")
        || lowered.contains("8bitdo")
        || lowered.contains("xbox")
        || lowered.contains("playstation")
        || lowered.contains("dualsense")
        || lowered.contains("dualshock")
        || lowered.contains("nintendo")
}

fn controller_display_name(raw: &str) -> String {
    raw.trim_start_matches("usb-")
        .trim_end_matches("-event-joystick")
        .trim_end_matches("-joystick")
        .replace(['_', '-'], " ")
}

fn controller_profile_status(device: Option<&ControllerDeviceStatus>) -> ControllerProfileStatus {
    let path = controller_profile_path().display().to_string();
    let exists = Path::new(&path).exists();
    ControllerProfileStatus {
        state: if exists { "saved" } else if device.is_some() { "ready to save" } else { "waiting for controller" }.to_string(),
        name: device.map(|d| format!("Default · {}", d.name)).unwrap_or_else(|| "Default".to_string()),
        path,
        bindings: saved_or_default_controller_bindings(),
    }
}

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
        ("Select", "button 6"),
        ("Start", "button 7"),
        ("D-pad Up", "hat 0 up"),
        ("D-pad Down", "hat 0 down"),
        ("D-pad Left", "hat 0 left"),
        ("D-pad Right", "hat 0 right"),
        ("Left Stick", "axis 0"),
        ("Right Stick", "axis 2"),
    ]
    .into_iter()
    .map(|(control, binding)| ControllerBindingStatus { control: control.to_string(), binding: binding.to_string(), pressed: false })
    .collect()
}


#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ControllerBindRequest {
    control: String,
    binding: Option<String>,
}

fn saved_or_default_controller_bindings() -> Vec<ControllerBindingStatus> {
    let path = controller_profile_path();
    let Ok(text) = fs::read_to_string(&path) else { return default_controller_bindings(); };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else { return default_controller_bindings(); };
    let items = value
        .get("tuples")
        .and_then(|v| v.as_array())
        .or_else(|| value.get("bindings").and_then(|v| v.as_array()));
    let Some(items) = items else { return default_controller_bindings(); };
    let mut bindings = default_controller_bindings();
    for item in items {
        let Some(control) = item.get("control").and_then(|v| v.as_str()) else { continue; };
        let input = item
            .get("input")
            .and_then(|v| v.as_str())
            .or_else(|| item.get("binding").and_then(|v| v.as_str()));
        let Some(input) = input else { continue; };
        upsert_binding(&mut bindings, control, input);
    }
    bindings
}

fn upsert_binding(bindings: &mut Vec<ControllerBindingStatus>, control: &str, input: &str) {
    let canonical = canonical_control_name(control);
    if let Some(existing) = bindings.iter_mut().find(|b| canonical_control_name(&b.control) == canonical) {
        existing.control = canonical;
        existing.binding = input.to_string();
        existing.pressed = false;
    } else {
        bindings.push(ControllerBindingStatus { control: canonical, binding: input.to_string(), pressed: false });
    }
}

fn canonical_control_name(control: &str) -> String {
    match control.trim().to_ascii_lowercase().as_str() {
        "left stick" | "left stick x" | "leftstick" => "Left Stick".to_string(),
        "right stick" | "right stick x" | "rightstick" => "Right Stick".to_string(),
        "d-pad up" | "dpad up" | "up" => "D-pad Up".to_string(),
        "d-pad down" | "dpad down" | "down" => "D-pad Down".to_string(),
        "d-pad left" | "dpad left" | "left" => "D-pad Left".to_string(),
        "d-pad right" | "dpad right" | "right" => "D-pad Right".to_string(),
        other => other.split_whitespace().map(|part| {
            if part.eq_ignore_ascii_case("d-pad") { "D-pad".to_string() }
            else if part.len() <= 2 { part.to_ascii_uppercase() }
            else { let mut chars = part.chars(); match chars.next() { Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(), None => String::new() } }
        }).collect::<Vec<_>>().join(" "),
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

fn capture_or_default_binding(status: &ControllerStatus, control: &str, explicit: Option<String>) -> String {
    if let Some(input) = explicit.filter(|value| !value.trim().is_empty()) {
        return input;
    }
    status
        .live_input
        .pressed
        .first()
        .or_else(|| status.live_input.axes.first())
        .map(|event| event.binding.clone())
        .unwrap_or_else(|| binding_for_control(control))
}

fn write_controller_profile(path: &Path, device_name: &str, handler: &str, bindings: &[ControllerBindingStatus]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() { fs::create_dir_all(parent)?; }
    let body = controller_profile_json(device_name, handler, bindings);
    fs::write(path, body)
}

fn controller_profile_json(device_name: &str, handler: &str, bindings: &[ControllerBindingStatus]) -> String {
    let tuples_json = bindings
        .iter()
        .map(|b| format!("    {{\"control\":\"{}\",\"input\":\"{}\"}}", json_escape(&canonical_control_name(&b.control)), json_escape(&b.binding)))
        .collect::<Vec<_>>()
        .join(",\n");
    format!("{{\n  \"schema\": \"arcadia.controller_profile.v1\",\n  \"name\": \"Default\",\n  \"device\": \"{}\",\n  \"handler\": \"{}\",\n  \"tuples\": [\n{}\n  ]\n}}\n", json_escape(device_name), json_escape(handler), tuples_json)
}

fn controller_profile_root() -> PathBuf {
    if let Ok(root) = env::var("ARCADIA_CONTROLLER_PROFILE_ROOT") {
        return PathBuf::from(root);
    }
    let primary = PathBuf::from("/var/lib/arcadia/controller-profiles");
    if primary.exists() || primary.parent().map(|p| p.exists() && is_writable_dir(p)).unwrap_or(false) {
        return primary;
    }
    PathBuf::from("/tmp/arcadia/controller-profiles")
}

fn virtual_controller_device(status: &ControllerStatus) -> (String, String) {
    status.devices.first()
        .map(|device| (device.name.clone(), device.handler.clone()))
        .unwrap_or_else(|| (status.primary_device.clone(), "virtual-arcadia-gamepad".to_string()))
}

fn read_controller_input(device: Option<&ControllerDeviceStatus>) -> ControllerInputStatus {
    let Some(device) = device else {
        return ControllerInputStatus { state: "waiting".to_string(), device: "No controller detected".to_string(), sample_path: String::new(), pressed: Vec::new(), axes: Vec::new() };
    };
    let sample_path = if device.path.contains("/dev/input/js") { device.path.clone() } else { first_js_path().unwrap_or_else(|| device.path.clone()) };
    let events = sample_js_events(&sample_path);
    let mut pressed = Vec::new();
    let mut axes = Vec::new();
    for (_, value, kind, number) in events {
        let event_type = kind & 0x7f;
        let is_initial_state = kind & 0x80 != 0;
        if event_type == 0x01 && value != 0 {
            pressed.push(ControllerBindingStatus { control: format!("Button {}", number), binding: format!("button {}", number), pressed: true });
        } else if event_type == 0x02 && !is_initial_state && value.abs() > 6000 {
            axes.push(ControllerBindingStatus { control: format!("Axis {}", number), binding: value.to_string(), pressed: true });
        }
    }
    ControllerInputStatus {
        state: if pressed.is_empty() && axes.is_empty() { "listening" } else { "active" }.to_string(),
        device: device.name.clone(),
        sample_path,
        pressed,
        axes,
    }
}

fn first_js_path() -> Option<String> {
    for entry in fs::read_dir("/dev/input").ok()?.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("js") {
            return Some(entry.path().display().to_string());
        }
    }
    None
}

fn sample_js_events(path: &str) -> Vec<(u32, i16, u8, u8)> {
    let Ok(output) = Command::new("/usr/bin/env")
        .args(["timeout", "0.18", "dd", &format!("if={}", path), "bs=8", "count=64", "status=none"])
        .output()
    else { return Vec::new(); };
    parse_js_events(&output.stdout)
}

fn parse_js_events(bytes: &[u8]) -> Vec<(u32, i16, u8, u8)> {
    let mut out = Vec::new();
    for chunk in bytes.chunks_exact(8) {
        let time = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        let value = i16::from_le_bytes([chunk[4], chunk[5]]);
        let kind = chunk[6];
        let number = chunk[7];
        out.push((time, value, kind, number));
    }
    out
}

fn emulator_controller_statuses() -> Vec<EmulatorControllerStatus> {
    vec![
        emulator_controller_status("RetroArch", "retroarch", "~/.config/retroarch/retroarch.cfg", "~/.config/retroarch/autoconfig", "libretro cores"),
        emulator_controller_status("Dolphin", "dolphin-emu", "~/.config/dolphin-emu/Dolphin.ini", "~/.config/dolphin-emu/Config/GCPadNew.ini", "GameCube / Wii"),
        emulator_controller_status("DuckStation", "duckstation-qt", "~/.config/duckstation/settings.ini", "~/.config/duckstation/inputprofiles", "PlayStation"),
        emulator_controller_status("PCSX2", "pcsx2-qt", "~/.config/PCSX2/inis/PCSX2.ini", "~/.config/PCSX2/inputprofiles", "PlayStation 2"),
        emulator_controller_status("PPSSPP", "PPSSPPSDL", "~/.config/ppsspp/PSP/SYSTEM/ppsspp.ini", "~/.config/ppsspp/PSP/SYSTEM/controls.ini", "PSP"),
    ]
}

fn emulator_controller_status(
    emulator: &str,
    command: &str,
    config_path: &str,
    mapping_path: &str,
    profile: &str,
) -> EmulatorControllerStatus {
    let installed = command_available(command);
    let configured = home_path_exists(config_path) || home_path_exists(mapping_path) || (emulator == "RetroArch" && retroarch_autoconfig_exists());
    let state = if installed && configured {
        "configured"
    } else if installed {
        "needs setup"
    } else {
        "not installed"
    };
    EmulatorControllerStatus {
        emulator: emulator.to_string(),
        command: command.to_string(),
        state: state.to_string(),
        config_path: config_path.to_string(),
        mapping_path: mapping_path.to_string(),
        profile: profile.to_string(),
    }
}

fn retroarch_autoconfig_exists() -> bool {
    Path::new("/home/steam/.config/retroarch/autoconfig/udev").exists()
        || Path::new("/home/owner/.config/retroarch/autoconfig/udev").exists()
}

fn command_available(command: &str) -> bool {
    Command::new("/usr/bin/env")
        .args(["sh", "-lc", &format!("command -v {} >/dev/null 2>&1", shell_quote(command))])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn home_path_exists(path: &str) -> bool {
    let candidates = if let Some(tail) = path.strip_prefix("~/") {
        vec![format!("/home/owner/{}", tail), format!("/home/steam/{}", tail)]
    } else {
        vec![path.to_string()]
    };
    candidates.iter().any(|expanded| Path::new(expanded).exists())
}

fn human_now_label() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("unix {}", secs)
}

async fn controllers_state_route(State(state): State<Arc<AppState>>) -> Json<ControllerStatus> {
    let status = console_status(&state);
    Json(status.controllers)
}

async fn controllers_input_route() -> Json<ControllerInputStatus> {
    let status = controller_status();
    Json(status.live_input)
}

async fn action_controllers_rescan() -> (StatusCode, Json<ConsoleActionResponse>) {
    let status = controller_status();
    let message = if status.detected_count > 0 {
        format!("Controller scan complete: {} gamepad(s) detected.", status.detected_count)
    } else {
        "Controller scan complete: no gamepad detected.".to_string()
    };
    (
        StatusCode::OK,
        Json(ConsoleActionResponse {
            ok: true,
            action: "controllers-rescan",
            command: "/proc/bus/input/devices",
            exit_code: Some(0),
            message,
            stdout: status.primary_device,
            stderr: String::new(),
        }),
    )
}

async fn action_controllers_test() -> (StatusCode, Json<ConsoleActionResponse>) {
    let status = controller_status();
    if status.detected_count == 0 {
        return console_action_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "controllers-test",
            "/dev/input",
            "No gamepad is detected. Pair or plug in a controller, then scan again.",
        );
    }
    let input = &status.live_input;
    let mut lines = vec![format!("{} · {}", input.device, input.sample_path)];
    if input.pressed.is_empty() && input.axes.is_empty() {
        lines.push("Listening. Hold a button or move a stick while the test is open.".to_string());
    } else {
        lines.extend(input.pressed.iter().map(|b| format!("{} pressed", b.control)));
        lines.extend(input.axes.iter().map(|a| format!("{} {}", a.control, a.binding)));
    }
    (
        StatusCode::OK,
        Json(ConsoleActionResponse {
            ok: true,
            action: "controllers-test",
            command: "/dev/input/js*",
            exit_code: Some(0),
            message: format!("Controller input test {}.", input.state),
            stdout: lines.join("\n"),
            stderr: String::new(),
        }),
    )
}

async fn action_controllers_save_profile() -> (StatusCode, Json<ConsoleActionResponse>) {
    let status = controller_status();
    let (device_name, handler) = virtual_controller_device(&status);
    let path = controller_profile_path();
    let bindings = saved_or_default_controller_bindings();
    if let Err(error) = write_controller_profile(&path, &device_name, &handler, &bindings) {
        return console_action_error(StatusCode::INTERNAL_SERVER_ERROR, "controllers-save-profile", "/var/lib/arcadia/controller-profiles/default.json", &format!("Could not save controller profile: {}", error));
    }
    (StatusCode::OK, Json(ConsoleActionResponse { ok: true, action: "controllers-save-profile", command: "/var/lib/arcadia/controller-profiles/default.json", exit_code: Some(0), message: "Controller profile saved.".to_string(), stdout: path.display().to_string(), stderr: String::new() }))
}

async fn action_controllers_bind(Json(payload): Json<ControllerBindRequest>) -> (StatusCode, Json<ConsoleActionResponse>) {
    let control = payload.control.trim();
    if control.is_empty() {
        return console_action_error(StatusCode::BAD_REQUEST, "controllers-bind", "/api/actions/controllers-bind", "Choose a controller button before binding.");
    }
    let status = controller_status();
    let (device_name, handler) = virtual_controller_device(&status);
    let binding = capture_or_default_binding(&status, control, payload.binding);
    let mut bindings = saved_or_default_controller_bindings();
    upsert_binding(&mut bindings, control, &binding);
    let path = controller_profile_path();
    if let Err(error) = write_controller_profile(&path, &device_name, &handler, &bindings) {
        return console_action_error(StatusCode::INTERNAL_SERVER_ERROR, "controllers-bind", "/var/lib/arcadia/controller-profiles/default.json", &format!("Could not write controller binding: {}", error));
    }
    (StatusCode::OK, Json(ConsoleActionResponse { ok: true, action: "controllers-bind", command: "/var/lib/arcadia/controller-profiles/default.json", exit_code: Some(0), message: format!("{} mapped to {}.", control, binding), stdout: binding, stderr: String::new() }))
}

async fn action_controllers_assign_retroarch() -> (StatusCode, Json<ConsoleActionResponse>) {
    action_controllers_assign_emulator("RetroArch")
}

async fn action_controllers_assign_dolphin() -> (StatusCode, Json<ConsoleActionResponse>) {
    action_controllers_assign_emulator("Dolphin")
}

async fn action_controllers_assign_duckstation() -> (StatusCode, Json<ConsoleActionResponse>) {
    action_controllers_assign_emulator("DuckStation")
}

async fn action_controllers_assign_pcsx2() -> (StatusCode, Json<ConsoleActionResponse>) {
    action_controllers_assign_emulator("PCSX2")
}

async fn action_controllers_assign_ppsspp() -> (StatusCode, Json<ConsoleActionResponse>) {
    action_controllers_assign_emulator("PPSSPP")
}

fn action_controllers_assign_emulator(emulator: &str) -> (StatusCode, Json<ConsoleActionResponse>) {
    let status = controller_status();
    let (device_name, handler) = virtual_controller_device(&status);
    let bindings = saved_or_default_controller_bindings();
    let installed = status.emulators.iter().find(|e| e.emulator == emulator).map(|e| e.state != "not installed").unwrap_or(false);
    match write_emulator_profile(emulator, &device_name, &handler, &bindings, installed, &controller_profile_root()) {
        Ok(path) => {
            let mode = if installed { "assigned" } else { "staged" };
            (StatusCode::OK, Json(ConsoleActionResponse { ok: true, action: "controllers-assign-emulator", command: "controller-profile-writer", exit_code: Some(0), message: format!("{} controller profile {}.", emulator, mode), stdout: path.display().to_string(), stderr: String::new() }))
        }
        Err(error) => console_action_error(StatusCode::INTERNAL_SERVER_ERROR, "controllers-assign-emulator", "controller-profile-writer", &format!("Could not write {} controller profile: {}", emulator, error)),
    }
}

fn write_emulator_profile(emulator: &str, device_name: &str, handler: &str, bindings: &[ControllerBindingStatus], installed: bool, root: &Path) -> std::io::Result<PathBuf> {
    let dir = root.join("emulators").join(safe_file_stem(emulator));
    fs::create_dir_all(&dir)?;
    let staged = dir.join("default-profile.txt");
    let body = emulator_profile_body(emulator, device_name, handler, bindings, installed);
    fs::write(&staged, &body)?;
    if installed {
        if emulator == "RetroArch" {
            let autoconfig = dir.join(format!("{}.cfg", safe_file_stem(device_name)));
            fs::write(&autoconfig, retroarch_autoconfig_from_bindings(device_name, bindings))?;
        } else {
            fs::write(dir.join("applied.ini"), &body)?;
        }
    }
    Ok(staged)
}

fn emulator_profile_body(emulator: &str, device_name: &str, handler: &str, bindings: &[ControllerBindingStatus], installed: bool) -> String {
    let mut lines = vec![
        format!("# Arcadia controller profile for {}", emulator),
        format!("device={}", device_name),
        format!("handler={}", handler),
        format!("mode={}", if installed { "assigned" } else { "staged-for-install" }),
    ];
    match emulator {
        "RetroArch" => lines.extend(bindings.iter().map(|b| format!("{}={}", retroarch_key_for_control(&b.control), retroarch_value_for_binding(&b.binding)))),
        "Dolphin" => lines.extend(bindings.iter().map(|b| format!("Arcadia/{}/{} = {}", device_name, b.control, dolphin_value_for_binding(&b.binding)))),
        "DuckStation" => lines.extend(bindings.iter().map(|b| format!("Pad1/{} = {}", b.control.replace(' ', ""), duckstation_value_for_binding(&b.binding)))),
        "PCSX2" => lines.extend(bindings.iter().map(|b| format!("Pad1_{} = {}", b.control.replace(' ', ""), pcsx2_value_for_binding(&b.binding)))),
        "PPSSPP" => lines.extend(bindings.iter().map(|b| format!("{} = {}", b.control.replace(' ', "_"), ppsspp_value_for_binding(&b.binding)))),
        _ => lines.extend(bindings.iter().map(|b| format!("{}={}", b.control, b.binding))),
    }
    format!("{}\n", lines.join("\n"))
}

fn retroarch_autoconfig_from_bindings(name: &str, bindings: &[ControllerBindingStatus]) -> String {
    let mut lines = vec![format!("input_device = \"{}\"", name), "input_driver = \"udev\"".to_string()];
    lines.extend(bindings.iter().map(|b| format!("{} = \"{}\"", retroarch_key_for_control(&b.control), retroarch_value_for_binding(&b.binding))));
    format!("{}\n", lines.join("\n"))
}

fn retroarch_key_for_control(control: &str) -> &'static str {
    match control {
        "A" => "input_a_btn", "B" => "input_b_btn", "X" => "input_x_btn", "Y" => "input_y_btn",
        "L1" => "input_l_btn", "R1" => "input_r_btn", "L2" => "input_l2_axis", "R2" => "input_r2_axis",
        "Start" => "input_start_btn", "Select" => "input_select_btn", "Left Stick" => "input_l3_btn", "Right Stick" => "input_r3_btn",
        "D-pad Up" => "input_up_btn", "D-pad Down" => "input_down_btn", "D-pad Left" => "input_left_btn", "D-pad Right" => "input_right_btn",
        _ => "input_menu_toggle_btn",
    }
}

fn binding_number(binding: &str) -> String {
    binding.split_whitespace().last().unwrap_or(binding).to_string()
}
fn retroarch_value_for_binding(binding: &str) -> String { if binding.starts_with("button ") { binding_number(binding) } else if binding.contains("up") { "h0up".to_string() } else if binding.contains("down") { "h0down".to_string() } else if binding.contains("left") { "h0left".to_string() } else if binding.contains("right") { "h0right".to_string() } else { binding.to_string() } }
fn dolphin_value_for_binding(binding: &str) -> String { format!("SDL/0/{}", binding.replace(' ', "_")) }
fn duckstation_value_for_binding(binding: &str) -> String { format!("SDL-0/{}", binding.replace(' ', "_")) }
fn pcsx2_value_for_binding(binding: &str) -> String { format!("SDL-0:{}", binding.replace(' ', "_")) }
fn ppsspp_value_for_binding(binding: &str) -> String { format!("SDL.{}", binding.replace(' ', "_")) }


fn controller_profile_path() -> PathBuf {
    controller_profile_root().join("default.json")
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

fn json_escape(value: &str) -> String { value.replace('\\', "\\\\").replace('"', "\\\"") }

fn safe_file_stem(value: &str) -> String {
    value.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect::<String>().trim_matches('-').to_string()
}
