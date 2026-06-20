fn hostname() -> String {
    command_stdout("hostname", &[]).unwrap_or_else(|| "homeconsole".to_string())
}

fn wifi_adapter_name() -> Option<String> {
    command_stdout("iw", &["dev"])
        .and_then(|text| {
            text.lines()
                .find_map(|line| line.trim().strip_prefix("Interface ").map(str::to_string))
        })
        .or_else(|| {
            fs::read_dir("/sys/class/net")
                .ok()?
                .flatten()
                .map(|e| e.file_name().to_string_lossy().to_string())
                .find(|n| n.starts_with("wl"))
        })
}

fn samba_available() -> bool {
    SAMBA_SERVICE_NAMES
        .iter()
        .any(|unit| service_state(unit) == "running")
        || tcp_port_listening(445)
}

fn surface_and_samba_status(
    canonical_url: &str,
    network: &NetworkStatus,
) -> (SurfaceStatus, SambaStatus) {
    let netbios = "HOMECONSOLE".to_string();
    let host = hostname();
    let share = "games".to_string();
    let samba_ok = samba_available() && Path::new(GAMES_ROOT).exists();
    let windows_unc = samba_ok.then(|| format!(r"\\{}\{}", netbios, share));
    let windows_unc_by_ip = (samba_ok && network.ip_address != "—")
        .then(|| format!(r"\\{}\{}", network.ip_address, share));
    let smb_url = samba_ok.then(|| format!("smb://{}/{}", host, share));
    let share_status = SambaShareStatus {
        name: share.clone(),
        purpose: "games".to_string(),
        windows_unc: windows_unc.clone(),
        windows_unc_by_ip: windows_unc_by_ip.clone(),
        smb_url: smb_url.clone(),
        smb_url_by_ip: (samba_ok && network.ip_address != "—")
            .then(|| format!("smb://{}/{}", network.ip_address, share)),
    };
    let shares = if samba_ok {
        vec![share_status]
    } else {
        Vec::new()
    };
    (
        SurfaceStatus {
            http: canonical_url.trim_end_matches('/').to_string(),
            mdns: format!("{}.local", host),
            smb: netbios,
            windows_unc,
            windows_unc_by_ip,
            smb_url,
            smb_url_by_ip: (samba_ok && network.ip_address != "—")
                .then(|| format!("smb://{}/{}", network.ip_address, share)),
        },
        SambaStatus {
            state: if samba_ok {
                "available"
            } else if SAMBA_SERVICE_NAMES
                .iter()
                .any(|unit| service_state(unit) == "unknown")
            {
                "unknown"
            } else {
                "disabled"
            }
            .to_string(),
            shares,
        },
    )
}

fn updates_status() -> UpdatesStatus {
    let current = env!("CARGO_PKG_VERSION").to_string();
    let deployed_sha_meta = fs::metadata("/var/lib/harmonia/state/arcadia.sha").ok();
    let deploy_meta = fs::metadata("/var/lib/harmonia/receipts/arcadia-latest/run.json").ok();
    let check_meta = fs::metadata("/var/lib/harmonia/receipts/arcadia-check-latest/run.json").ok();
    let deploy_is_fresher = match (&deploy_meta, &check_meta) {
        (Some(deploy), Some(check)) => deploy.modified().ok() >= check.modified().ok(),
        (Some(_), None) => true,
        _ => false,
    };
    let deployed_ok = fs::read_to_string("/var/lib/harmonia/receipts/arcadia-latest/run.json")
        .map(|text| text.contains("\"ok\":true") || text.contains("\"ok\": true"))
        .unwrap_or(false);
    let state = if deployed_sha_meta.is_some() && deployed_ok && deploy_is_fresher {
        "current"
    } else if let Ok(text) =
        fs::read_to_string("/var/lib/harmonia/receipts/arcadia-check-latest/run.json")
    {
        if text.contains("\"update_available\":true") || text.contains("\"update_available\": true")
        {
            "available"
        } else if text.contains("\"ok\":false") || text.contains("\"ok\": false") {
            "error"
        } else {
            "current"
        }
    } else {
        "unknown"
    };
    UpdatesStatus {
        state: state.to_string(),
        current_version: current,
        available_version: None,
    }
}

fn runtime_status(started_unix: u64) -> RuntimeStatus {
    let machine_uptime_seconds = fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|uptime| uptime.split_whitespace().next().map(str::to_string))
        .and_then(|seconds| seconds.split('.').next().unwrap_or("0").parse::<u64>().ok())
        .unwrap_or(0);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(started_unix);
    let arcadia_uptime_seconds = now.saturating_sub(started_unix);
    RuntimeStatus {
        machine_uptime: format_duration(machine_uptime_seconds),
        machine_uptime_seconds,
        arcadia_uptime: format_duration(arcadia_uptime_seconds),
        arcadia_uptime_seconds,
    }
}

fn format_duration(total_seconds: u64) -> String {
    let days = total_seconds / 86_400;
    let hours = (total_seconds % 86_400) / 3_600;
    let minutes = (total_seconds % 3_600) / 60;
    if days > 0 {
        format!("{days}d {hours}h {minutes}m")
    } else if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    }
}

fn gui_pin_status() -> GuiPinStatus {
    let reset_helper_present = helper_exists(GUI_PIN_RESET_HELPER);
    GuiPinStatus {
        pin_required: gui_pin_required(),
        state_path: GUI_PIN_STATE_PATH,
        access_helper_present: helper_exists(GUI_PIN_ACCESS_HELPER),
        pin_change_helper_present: helper_exists(GUI_PIN_CHANGE_HELPER),
        pin_reset_helper_present: reset_helper_present,
        pin_storage: "keyman-redacted",
        default_reset_available: reset_helper_present,
    }
}

fn gui_pin_required() -> bool {
    fs::read_to_string(GUI_PIN_STATE_PATH)
        .map(|state| {
            let normalized = state.to_ascii_lowercase();
            normalized.contains("pin_required=true")
                || normalized.contains("pin_required: true")
                || normalized.contains("\"pin_required\":true")
                || normalized.trim() == "required"
                || normalized.trim() == "true"
        })
        .unwrap_or(false)
}

fn helper_exists(path: &str) -> bool {
    Path::new(path).exists()
}
