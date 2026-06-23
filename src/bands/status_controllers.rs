include!("controller_writers/mod.rs");

fn controller_status() -> ControllerStatus {
    controller_status_options(true, true)
}

fn controller_status_api() -> ControllerStatus {
    controller_status_options(false, false)
}

fn controller_status_options(include_live_input: bool, include_emulators: bool) -> ControllerStatus {
    let devices = controller_devices();
    let detected_count = devices.len();
    let recovery = controller_recovery_status(&devices);
    let state = if detected_count > 0 { "connected".to_string() } else { recovery.state.clone() };
    let controller_pool = controller_pool_entries(&devices);
    let active_controller_id = active_controller_id();
    let (active_name, _, _, active_bindings) = active_controller_device_tuple();
    let active_tuning = tuning_for_active_controller();
    let primary_device = if active_name.is_empty() || active_name == "No controller selected" {
        devices
            .first()
            .map(|device| device.name.clone())
            .unwrap_or_else(|| recovery.title.clone())
    } else {
        active_name
    };
    let active_device = active_connected_device(&devices, &active_controller_id);
    let profile = controller_profile_status(
        active_device.as_ref(),
        &primary_device,
        &active_bindings,
        active_tuning,
    );
    let live_input = if include_live_input {
        read_controller_input(active_device.as_ref().or(devices.first()))
    } else {
        controller_live_input_idle(&primary_device)
    };
    let emulators = if include_emulators {
        emulator_controller_statuses()
    } else {
        Vec::new()
    };
    ControllerStatus {
        state,
        detected_count,
        primary_device,
        active_controller_id,
        last_scan: human_now_label(),
        recovery,
        devices,
        controller_pool,
        profile,
        profile_presets: controller_profile_presets(),
        live_input,
        emulators,
    }
}

fn controller_live_input_idle(device_label: &str) -> ControllerInputStatus {
    ControllerInputStatus {
        state: "idle".to_string(),
        device: device_label.to_string(),
        sample_path: String::new(),
        pressed: Vec::new(),
        axes: Vec::new(),
    }
}

fn active_connected_device(
    devices: &[ControllerDeviceStatus],
    active_controller_id: &str,
) -> Option<ControllerDeviceStatus> {
    devices
        .iter()
        .find(|device| controller_id_for_device(device) == active_controller_id)
        .cloned()
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
    [
        (
            "Default",
            "Xbox-style labels",
            "Standard A/B/X/Y and stick names. Best starting point for most gamepads and emulators.",
            "active",
        ),
        (
            "Nintendo",
            "Swapped A ↔ B",
            "Swaps A and B (and X and Y) so on-screen Nintendo names match what you press.",
            "available",
        ),
        (
            "PlayStation",
            "△ ○ □ ✕ names",
            "Uses PlayStation-style face names (cross, circle, square, triangle) on the virtual pad.",
            "available",
        ),
        (
            "Arcade",
            "D-pad forward",
            "Highlights D-pad mapping for arcade cores and digital-first platforms.",
            "available",
        ),
    ]
    .into_iter()
    .map(|(name, layout, description, state)| ControllerProfilePresetStatus {
        name: name.to_string(),
        layout: layout.to_string(),
        description: description.to_string(),
        state: state.to_string(),
    })
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

fn controller_profile_status(
    device: Option<&ControllerDeviceStatus>,
    active_name: &str,
    bindings: &[ControllerBindingStatus],
    tuning: ControllerTuningStatus,
) -> ControllerProfileStatus {
    let path = controller_library_path().display().to_string();
    let exists = Path::new(&path).exists();
    ControllerProfileStatus {
        state: if exists { "saved" } else if device.is_some() { "ready to save" } else { "waiting for controller" }.to_string(),
        name: if active_name.is_empty() {
            "No controller selected".to_string()
        } else {
            active_name.to_string()
        },
        path,
        bindings: bindings.to_vec(),
        tuning,
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ControllerBindRequest {
    control: String,
    binding: Option<String>,
    controller_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ControllerProfileApplyRequest {
    profile: String,
    controller_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ControllerSelectRequest {
    controller_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ControllerScopedRequest {
    controller_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ControllerTuningSaveRequest {
    controller_id: Option<String>,
    left_stick_deadzone: Option<f32>,
    right_stick_deadzone: Option<f32>,
    left_stick_sensitivity: Option<f32>,
    right_stick_sensitivity: Option<f32>,
}

fn resolve_controller_id(explicit: Option<String>) -> String {
    explicit
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(active_controller_id)
}

fn controller_device_for_id(status: &ControllerStatus, controller_id: &str) -> (String, String) {
    if let Some(entry) = status
        .controller_pool
        .iter()
        .find(|entry| entry.id == controller_id)
    {
        return (entry.name.clone(), entry.handler.clone());
    }
    if let Some(device) = status
        .devices
        .iter()
        .find(|device| controller_id_for_device(device) == controller_id)
    {
        return (device.name.clone(), device.handler.clone());
    }
    let (_, name, handler, _) = active_controller_device_tuple();
    (name, handler)
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

fn virtual_controller_device(status: &ControllerStatus, controller_id: &str) -> (String, String) {
    controller_device_for_id(status, controller_id)
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

async fn controllers_state_route() -> Json<ControllerStatus> {
    Json(controller_status_api())
}

async fn controllers_input_route() -> Json<ControllerInputStatus> {
    let devices = controller_devices();
    let active_controller_id = active_controller_id();
    let active_device = active_connected_device(&devices, &active_controller_id);
    Json(read_controller_input(active_device.as_ref().or(devices.first())))
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

async fn action_controllers_save_profile(
    Json(payload): Json<ControllerScopedRequest>,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    let status = controller_status();
    let controller_id = resolve_controller_id(payload.controller_id);
    if controller_id.is_empty() {
        return console_action_error(StatusCode::BAD_REQUEST, "controllers-save-profile", "/api/actions/controllers-save-profile", "Choose a controller before saving.");
    }
    let (device_name, handler) = virtual_controller_device(&status, &controller_id);
    let bindings = bindings_for_controller_id(&controller_id);
    let path = controller_library_path();
    if let Err(error) = save_bindings_for_controller(&controller_id, &bindings, None) {
        return console_action_error(StatusCode::INTERNAL_SERVER_ERROR, "controllers-save-profile", "/var/lib/arcadia/controller-profiles/library.json", &format!("Could not save controller profile: {}", error));
    }
    let tuning = tuning_for_controller_id(&controller_id);
    let ramrod_detail = match ramrod_controller_profiles(
        &device_name,
        &handler,
        &bindings,
        &tuning,
        &status.emulators,
    ) {
        Ok(receipt) => ramrod_stdout(&receipt),
        Err(error) => format!("profile saved; ramrod deferred: {error}"),
    };
    (StatusCode::OK, Json(ConsoleActionResponse { ok: true, action: "controllers-save-profile", command: "/var/lib/arcadia/controller-profiles/library.json", exit_code: Some(0), message: format!("{} layout saved and ramrodded.", device_name), stdout: format!("{}\n{}", path.display(), ramrod_detail), stderr: String::new() }))
}

async fn action_controllers_apply_profile(Json(payload): Json<ControllerProfileApplyRequest>) -> (StatusCode, Json<ConsoleActionResponse>) {
    let profile = payload.profile.trim();
    if profile.is_empty() {
        return console_action_error(StatusCode::BAD_REQUEST, "controllers-apply-profile", "/api/actions/controllers-apply-profile", "Choose a controller profile.");
    }
    let status = controller_status();
    let controller_id = resolve_controller_id(payload.controller_id);
    if controller_id.is_empty() {
        return console_action_error(StatusCode::BAD_REQUEST, "controllers-apply-profile", "/api/actions/controllers-apply-profile", "Choose a controller before applying a layout style.");
    }
    let (device_name, handler) = virtual_controller_device(&status, &controller_id);
    let bindings = controller_bindings_for_profile(profile);
    let path = controller_library_path();
    if let Err(error) = save_bindings_for_controller(&controller_id, &bindings, Some(profile)) {
        return console_action_error(StatusCode::INTERNAL_SERVER_ERROR, "controllers-apply-profile", "/var/lib/arcadia/controller-profiles/library.json", &format!("Could not apply controller profile: {}", error));
    }
    let tuning = tuning_for_controller_id(&controller_id);
    let ramrod_detail = match ramrod_controller_profiles(
        &device_name,
        &handler,
        &bindings,
        &tuning,
        &status.emulators,
    ) {
        Ok(receipt) => ramrod_stdout(&receipt),
        Err(error) => format!("profile applied; ramrod deferred: {error}"),
    };
    (StatusCode::OK, Json(ConsoleActionResponse { ok: true, action: "controllers-apply-profile", command: "/var/lib/arcadia/controller-profiles/library.json", exit_code: Some(0), message: format!("{} layout applied to {}.", profile, device_name), stdout: format!("{}\n{}", path.display(), ramrod_detail), stderr: String::new() }))
}

async fn action_controllers_bind(Json(payload): Json<ControllerBindRequest>) -> (StatusCode, Json<ConsoleActionResponse>) {
    let control = payload.control.trim();
    if control.is_empty() {
        return console_action_error(StatusCode::BAD_REQUEST, "controllers-bind", "/api/actions/controllers-bind", "Choose a controller button before binding.");
    }
    let status = controller_status();
    let controller_id = resolve_controller_id(payload.controller_id);
    if controller_id.is_empty() {
        return console_action_error(StatusCode::BAD_REQUEST, "controllers-bind", "/api/actions/controllers-bind", "Choose a controller before binding.");
    }
    let (device_name, handler) = virtual_controller_device(&status, &controller_id);
    let binding = capture_or_default_binding(&status, control, payload.binding);
    let mut bindings = bindings_for_controller_id(&controller_id);
    upsert_binding(&mut bindings, control, &binding);
    if let Err(error) = save_bindings_for_controller(&controller_id, &bindings, None) {
        return console_action_error(StatusCode::INTERNAL_SERVER_ERROR, "controllers-bind", "/var/lib/arcadia/controller-profiles/library.json", &format!("Could not write controller binding: {}", error));
    }
    let tuning = tuning_for_controller_id(&controller_id);
    let ramrod_note = match ramrod_controller_profiles(
        &device_name,
        &handler,
        &bindings,
        &tuning,
        &status.emulators,
    ) {
        Ok(_) => " Tuple ramrodded to emulator strata.".to_string(),
        Err(_) => String::new(),
    };
    (StatusCode::OK, Json(ConsoleActionResponse { ok: true, action: "controllers-bind", command: "/var/lib/arcadia/controller-profiles/library.json", exit_code: Some(0), message: format!("{} mapped to {} on {}.{ramrod_note}", control, binding, device_name), stdout: binding, stderr: String::new() }))
}

async fn action_controllers_save_tuning(
    Json(payload): Json<ControllerTuningSaveRequest>,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    let status = controller_status();
    let controller_id = resolve_controller_id(payload.controller_id);
    if controller_id.is_empty() {
        return console_action_error(
            StatusCode::BAD_REQUEST,
            "controllers-save-tuning",
            "/api/actions/controllers-save-tuning",
            "Choose a controller before saving stick tuning.",
        );
    }
    let mut tuning = tuning_for_controller_id(&controller_id);
    if let Some(value) = payload.left_stick_deadzone {
        tuning.left_stick_deadzone = value;
    }
    if let Some(value) = payload.right_stick_deadzone {
        tuning.right_stick_deadzone = value;
    }
    if let Some(value) = payload.left_stick_sensitivity {
        tuning.left_stick_sensitivity = value;
    }
    if let Some(value) = payload.right_stick_sensitivity {
        tuning.right_stick_sensitivity = value;
    }
    match save_tuning_for_controller(&controller_id, tuning) {
        Ok((device_name, handler, bindings, saved_tuning)) => {
            let ramrod_detail = match ramrod_controller_profiles(
                &device_name,
                &handler,
                &bindings,
                &saved_tuning,
                &status.emulators,
            ) {
                Ok(receipt) => ramrod_stdout(&receipt),
                Err(error) => format!("tuning saved; ramrod deferred: {error}"),
            };
            (
                StatusCode::OK,
                Json(ConsoleActionResponse {
                    ok: true,
                    action: "controllers-save-tuning",
                    command: "/var/lib/arcadia/controller-profiles/library.json",
                    exit_code: Some(0),
                    message: format!("{device_name} stick dampening applied."),
                    stdout: ramrod_detail,
                    stderr: String::new(),
                }),
            )
        }
        Err(error) => console_action_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "controllers-save-tuning",
            "/api/actions/controllers-save-tuning",
            &format!("Could not save controller tuning: {error}"),
        ),
    }
}

async fn action_controllers_select(Json(payload): Json<ControllerSelectRequest>) -> (StatusCode, Json<ConsoleActionResponse>) {
    let controller_id = payload.controller_id.trim();
    if controller_id.is_empty() {
        return console_action_error(StatusCode::BAD_REQUEST, "controllers-select", "/api/actions/controllers-select", "Choose a controller from your library.");
    }
    match set_active_controller_id(controller_id) {
        Ok(()) => {
            let status = controller_status_api();
            let entry = status
                .controller_pool
                .iter()
                .find(|entry| entry.id == controller_id)
                .map(|entry| entry.name.clone())
                .unwrap_or_else(|| controller_id.to_string());
            (
                StatusCode::OK,
                Json(ConsoleActionResponse {
                    ok: true,
                    action: "controllers-select",
                    command: "/var/lib/arcadia/controller-profiles/library.json",
                    exit_code: Some(0),
                    message: format!("{entry} selected for mapping."),
                    stdout: controller_id.to_string(),
                    stderr: String::new(),
                }),
            )
        }
        Err(error) => console_action_error(
            StatusCode::NOT_FOUND,
            "controllers-select",
            "/api/actions/controllers-select",
            &format!("Could not select controller: {error}"),
        ),
    }
}

async fn action_controllers_assign_retroarch(
    Json(payload): Json<ControllerScopedRequest>,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    action_controllers_assign_emulator("RetroArch", payload.controller_id)
}

async fn action_controllers_assign_dolphin(
    Json(payload): Json<ControllerScopedRequest>,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    action_controllers_assign_emulator("Dolphin", payload.controller_id)
}

async fn action_controllers_assign_duckstation(
    Json(payload): Json<ControllerScopedRequest>,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    action_controllers_assign_emulator("DuckStation", payload.controller_id)
}

async fn action_controllers_assign_pcsx2(
    Json(payload): Json<ControllerScopedRequest>,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    action_controllers_assign_emulator("PCSX2", payload.controller_id)
}

async fn action_controllers_assign_ppsspp(
    Json(payload): Json<ControllerScopedRequest>,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    action_controllers_assign_emulator("PPSSPP", payload.controller_id)
}

async fn action_controllers_ramrod_all() -> (StatusCode, Json<ConsoleActionResponse>) {
    let status = controller_status();
    let controller_id = active_controller_id();
    let (device_name, handler) = virtual_controller_device(&status, &controller_id);
    let bindings = bindings_for_controller_id(&controller_id);
    let tuning = tuning_for_controller_id(&controller_id);
    match ramrod_controller_profiles(
        &device_name,
        &handler,
        &bindings,
        &tuning,
        &status.emulators,
    ) {
        Ok(receipt) => (
            StatusCode::OK,
            Json(ConsoleActionResponse {
                ok: true,
                action: "controllers-ramrod-all",
                command: "controller-profile-ramrod",
                exit_code: Some(0),
                message: format!(
                    "Tuple profile ramrodded across {} emulator strata.",
                    receipt.entries.len()
                ),
                stdout: ramrod_stdout(&receipt),
                stderr: String::new(),
            }),
        ),
        Err(error) => console_action_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "controllers-ramrod-all",
            "controller-profile-ramrod",
            &format!("Could not ramrod controller profile: {error}"),
        ),
    }
}

fn action_controllers_assign_emulator(
    emulator: &str,
    explicit_controller_id: Option<String>,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    let status = controller_status();
    let controller_id = resolve_controller_id(explicit_controller_id);
    if controller_id.is_empty() {
        return console_action_error(
            StatusCode::BAD_REQUEST,
            "controllers-assign-emulator",
            "controller-profile-writer",
            "Select a controller from Your controllers before pushing a mapping.",
        );
    }
    let (device_name, handler) = virtual_controller_device(&status, &controller_id);
    let bindings = bindings_for_controller_id(&controller_id);
    let tuning = tuning_for_controller_id(&controller_id);
    let installed = status
        .emulators
        .iter()
        .find(|e| e.emulator == emulator)
        .map(|e| e.state != "not installed")
        .unwrap_or(false);
    match write_emulator_profile(
        emulator,
        &device_name,
        &handler,
        &bindings,
        &tuning,
        installed,
        &controller_profile_root(),
    ) {
        Ok(path) => {
            let mode = if installed { "pushed" } else { "staged" };
            (
                StatusCode::OK,
                Json(ConsoleActionResponse {
                    ok: true,
                    action: "controllers-assign-emulator",
                    command: "controller-profile-writer",
                    exit_code: Some(0),
                    message: format!("{device_name} mapping {mode} to {emulator}."),
                    stdout: path.display().to_string(),
                    stderr: String::new(),
                }),
            )
        }
        Err(error) => console_action_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "controllers-assign-emulator",
            "controller-profile-writer",
            &format!("Could not write {emulator} controller profile: {error}"),
        ),
    }
}


