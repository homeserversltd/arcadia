fn controller_status() -> ControllerStatus {
    let devices = controller_devices();
    let detected_count = devices.len();
    let state = if detected_count > 0 { "ready" } else { "waiting" }.to_string();
    let primary_device = devices
        .first()
        .map(|device| device.name.clone())
        .unwrap_or_else(|| "No controller detected".to_string());
    ControllerStatus {
        state,
        detected_count,
        primary_device,
        last_scan: human_now_label(),
        devices,
        emulators: emulator_controller_statuses(),
    }
}

fn controller_devices() -> Vec<ControllerDeviceStatus> {
    let mut devices = controllers_from_proc_bus_input();
    if devices.is_empty() {
        devices = controllers_from_dev_input();
    }
    devices.sort_by(|a, b| a.handler.cmp(&b.handler).then(a.name.cmp(&b.name)));
    devices.dedup_by(|a, b| a.handler == b.handler && a.path == b.path);
    devices
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
        if !(handlers.contains("js") || handlers.contains("event")) {
            continue;
        }
        let lowered = name.to_ascii_lowercase();
        let controllerish = lowered.contains("gamepad")
            || lowered.contains("controller")
            || lowered.contains("xbox")
            || lowered.contains("playstation")
            || lowered.contains("dualshock")
            || lowered.contains("dualsense")
            || handlers.contains("js");
        if !controllerish {
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
            kind: if handler.starts_with("js") { "joystick" } else { "input" }.to_string(),
            path,
            state: "detected".to_string(),
        });
    }
    out
}

fn controllers_from_dev_input() -> Vec<ControllerDeviceStatus> {
    let mut out = Vec::new();
    for root in ["/dev/input/by-id", "/dev/input"] {
        let Ok(entries) = fs::read_dir(root) else { continue; };
        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();
            let lowered = file_name.to_ascii_lowercase();
            if !(lowered.contains("gamepad")
                || lowered.contains("controller")
                || lowered.contains("joystick")
                || lowered.starts_with("js"))
            {
                continue;
            }
            out.push(ControllerDeviceStatus {
                name: controller_display_name(&file_name),
                handler: file_name.clone(),
                kind: if lowered.starts_with("js") { "joystick" } else { "input" }.to_string(),
                path: path.display().to_string(),
                state: "detected".to_string(),
            });
        }
    }
    out
}

fn controller_display_name(raw: &str) -> String {
    raw.trim_start_matches("usb-")
        .trim_end_matches("-event-joystick")
        .trim_end_matches("-joystick")
        .replace(['_', '-'], " ")
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
    let configured = home_path_exists(config_path) || home_path_exists(mapping_path);
    let state = if installed && configured {
        "configured"
    } else if installed {
        "installed"
    } else {
        "unavailable"
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
    let expanded = path.strip_prefix("~/").map(|tail| format!("/home/owner/{}", tail)).unwrap_or_else(|| path.to_string());
    Path::new(&expanded).exists()
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

async fn action_controllers_rescan() -> (StatusCode, Json<ConsoleActionResponse>) {
    let status = controller_status();
    let message = if status.detected_count > 0 {
        format!("Controller scan complete: {} device(s) detected.", status.detected_count)
    } else {
        "Controller scan complete: no gamepad detected.".to_string()
    };
    (
        StatusCode::OK,
        Json(ConsoleActionResponse {
            ok: true,
            action: "controllers-rescan",
            command: "/dev/input",
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
            "No controller is detected. Pair or plug in a controller, then scan again.",
        );
    }
    let stdout = status
        .devices
        .iter()
        .map(|device| format!("{} · {} · {}", device.name, device.handler, device.path))
        .collect::<Vec<_>>()
        .join("\n");
    (
        StatusCode::OK,
        Json(ConsoleActionResponse {
            ok: true,
            action: "controllers-test",
            command: "/proc/bus/input/devices",
            exit_code: Some(0),
            message: format!("Controller input surface ready: {} device(s).", status.detected_count),
            stdout,
            stderr: String::new(),
        }),
    )
}
