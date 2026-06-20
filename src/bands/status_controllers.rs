fn controller_status() -> ControllerStatus {
    let devices = controller_devices();
    let detected_count = devices.len();
    let state = if detected_count > 0 { "ready" } else { "waiting" }.to_string();
    let primary_device = devices
        .first()
        .map(|device| device.name.clone())
        .unwrap_or_else(|| "No gamepad detected".to_string());
    let profile = controller_profile_status(devices.first());
    let live_input = read_controller_input(devices.first());
    ControllerStatus {
        state,
        detected_count,
        primary_device,
        last_scan: human_now_label(),
        devices,
        profile,
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
            name,
            handler: handler.to_string(),
            kind: if handler.starts_with("js") { "joystick" } else { "gamepad" }.to_string(),
            path,
            state: "detected".to_string(),
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
        bindings: default_controller_bindings(),
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
        ("Select", "button 6"),
        ("Start", "button 7"),
        ("Left stick X", "axis 0"),
        ("Left stick Y", "axis 1"),
        ("D-pad", "hat 0"),
    ]
    .into_iter()
    .map(|(control, binding)| ControllerBindingStatus { control: control.to_string(), binding: binding.to_string(), pressed: false })
    .collect()
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
    if status.detected_count == 0 {
        return console_action_error(StatusCode::SERVICE_UNAVAILABLE, "controllers-save-profile", "/var/lib/arcadia/controller-profiles", "No gamepad is detected. Scan again after plugging in a controller.");
    }
    let path = controller_profile_path();
    if let Some(parent) = path.parent() {
        if let Err(error) = fs::create_dir_all(parent) {
            return console_action_error(StatusCode::INTERNAL_SERVER_ERROR, "controllers-save-profile", "/var/lib/arcadia/controller-profiles", &format!("Could not create controller profile directory: {}", error));
        }
    }
    let bindings = default_controller_bindings()
        .into_iter()
        .map(|b| format!("    {{\"control\":\"{}\",\"binding\":\"{}\"}}", b.control, b.binding))
        .collect::<Vec<_>>()
        .join(",\n");
    let body = format!("{{\n  \"schema\": \"arcadia.controller_profile.v1\",\n  \"name\": \"Default\",\n  \"device\": \"{}\",\n  \"handler\": \"{}\",\n  \"bindings\": [\n{}\n  ]\n}}\n", json_escape(&status.devices[0].name), json_escape(&status.devices[0].handler), bindings);
    if let Err(error) = fs::write(&path, body) {
        return console_action_error(StatusCode::INTERNAL_SERVER_ERROR, "controllers-save-profile", "/var/lib/arcadia/controller-profiles/default.json", &format!("Could not save controller profile: {}", error));
    }
    (StatusCode::OK, Json(ConsoleActionResponse { ok: true, action: "controllers-save-profile", command: "/var/lib/arcadia/controller-profiles/default.json", exit_code: Some(0), message: "Controller profile saved.".to_string(), stdout: path.display().to_string(), stderr: String::new() }))
}

async fn action_controllers_assign_retroarch() -> (StatusCode, Json<ConsoleActionResponse>) {
    let status = controller_status();
    if status.detected_count == 0 {
        return console_action_error(StatusCode::SERVICE_UNAVAILABLE, "controllers-assign-retroarch", "/home/steam/.config/retroarch/autoconfig/udev", "No gamepad is detected. Scan again after plugging in a controller.");
    }
    let device = &status.devices[0];
    let dir = Path::new("/home/steam/.config/retroarch/autoconfig/udev");
    if let Err(error) = fs::create_dir_all(dir) {
        return console_action_error(StatusCode::INTERNAL_SERVER_ERROR, "controllers-assign-retroarch", "/home/steam/.config/retroarch/autoconfig/udev", &format!("Could not create RetroArch autoconfig directory: {}", error));
    }
    let cfg_path = dir.join(format!("{}.cfg", safe_file_stem(&device.name)));
    let cfg = retroarch_autoconfig(&device.name);
    if let Err(error) = fs::write(&cfg_path, cfg) {
        return console_action_error(StatusCode::INTERNAL_SERVER_ERROR, "controllers-assign-retroarch", "/home/steam/.config/retroarch/autoconfig/udev", &format!("Could not write RetroArch controller config: {}", error));
    }
    let retroarch_cfg = Path::new("/home/steam/.config/retroarch/retroarch.cfg");
    if let Some(parent) = retroarch_cfg.parent() { let _ = fs::create_dir_all(parent); }
    let _ = ensure_retroarch_line(retroarch_cfg, "input_joypad_driver", "udev");
    let _ = ensure_retroarch_line(retroarch_cfg, "input_player1_analog_dpad_mode", "1");
    (StatusCode::OK, Json(ConsoleActionResponse { ok: true, action: "controllers-assign-retroarch", command: "/home/steam/.config/retroarch/autoconfig/udev", exit_code: Some(0), message: "RetroArch controller profile assigned.".to_string(), stdout: cfg_path.display().to_string(), stderr: String::new() }))
}

fn controller_profile_path() -> PathBuf {
    let primary = PathBuf::from("/var/lib/arcadia/controller-profiles/default.json");
    if primary.exists() || primary.parent().map(|p| p.exists() && is_writable_dir(p)).unwrap_or(false) {
        return primary;
    }
    PathBuf::from("/tmp/arcadia/controller-profiles/default.json")
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

fn retroarch_autoconfig(name: &str) -> String {
    format!(r#"input_device = "{}"
input_driver = "udev"
input_vendor_id = "11720"
input_product_id = "12554"
input_b_btn = "0"
input_y_btn = "2"
input_select_btn = "6"
input_start_btn = "7"
input_up_axis = "-1"
input_down_axis = "+1"
input_left_axis = "-0"
input_right_axis = "+0"
input_a_btn = "1"
input_x_btn = "3"
input_l_btn = "4"
input_r_btn = "5"
input_l2_axis = "+2"
input_r2_axis = "+5"
input_l3_btn = "9"
input_r3_btn = "10"
input_up_btn = "h0up"
input_down_btn = "h0down"
input_left_btn = "h0left"
input_right_btn = "h0right"
input_menu_toggle_btn = "8"
"#, name)
}

fn ensure_retroarch_line(path: &Path, key: &str, value: &str) -> std::io::Result<()> {
    let existing = fs::read_to_string(path).unwrap_or_default();
    let wanted = format!("{} = \"{}\"", key, value);
    let mut found = false;
    let mut lines = Vec::new();
    for line in existing.lines() {
        if line.trim_start().starts_with(&format!("{} =", key)) {
            lines.push(wanted.clone());
            found = true;
        } else {
            lines.push(line.to_string());
        }
    }
    if !found { lines.push(wanted); }
    fs::write(path, format!("{}\n", lines.join("\n")))
}
