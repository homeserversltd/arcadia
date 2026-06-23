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
    let suite_receipt = "/var/lib/harmonia/receipts/homeconsole-latest/run.json";
    let check_receipt = "/var/lib/harmonia/receipts/homeconsole-check-latest/run.json";
    let arcadia_receipt = "/var/lib/harmonia/receipts/arcadia-gui-latest/run.json";
    let profile = harmonia_profile_modules();
    let suite = read_json_value(suite_receipt);
    let check = read_json_value(check_receipt);
    let receipt = suite
        .clone()
        .or_else(|| check.clone())
        .unwrap_or(serde_json::Value::Null);
    let arcadia = read_json_value(arcadia_receipt);
    let suite_ok = suite
        .as_ref()
        .and_then(|v| receipt_bool(v, "suite_ok").or_else(|| receipt_bool(v, "ok")))
        .unwrap_or(false);
    let suite_signal = suite
        .as_ref()
        .and_then(|v| receipt_string(v, "first_missing_signal"))
        .unwrap_or_else(|| {
            if suite.is_some() {
                "unknown".to_string()
            } else {
                "receipt-missing".to_string()
            }
        });
    let suite_changed = suite
        .as_ref()
        .and_then(|v| receipt_bool(v, "changed"))
        .unwrap_or(false);
    let check_ok = check
        .as_ref()
        .and_then(|v| receipt_bool(v, "suite_ok").or_else(|| receipt_bool(v, "ok")))
        .unwrap_or(false);
    let check_changed = check
        .as_ref()
        .and_then(|v| receipt_bool(v, "changed"))
        .unwrap_or(false);
    let check_missing_signal = check
        .as_ref()
        .and_then(|v| receipt_string(v, "first_missing_signal"))
        .unwrap_or_else(|| {
            if check.is_some() {
                "unknown".to_string()
            } else {
                "not-checked".to_string()
            }
        });
    let first_missing_signal = if check.is_some() && !check_ok && check_missing_signal != "none" {
        check_missing_signal.clone()
    } else if suite.is_some() && !suite_ok && suite_signal != "none" {
        suite_signal.clone()
    } else if check_ok || suite_ok {
        "none".to_string()
    } else if check.is_none() && suite.is_none() {
        "receipt-missing".to_string()
    } else {
        suite_signal.clone()
    };
    let profile_id = receipt_string(&receipt, "profile_id").unwrap_or_else(|| "homeconsole".to_string());
    let identity = receipt_string(&receipt, "identity").unwrap_or_else(|| "homeconsole".to_string());
    let module_count = receipt_usize(&receipt, "module_count").unwrap_or(profile.len());
    let operation_count = receipt_usize(&receipt, "operation_count").unwrap_or(0);
    let arcadia_ok = arcadia.as_ref().and_then(|v| receipt_bool(v, "ok")).unwrap_or(false);
    let state = if check.is_some() && !check_ok && check_missing_signal != "none" {
        "repair_pending"
    } else if suite.is_some() && !suite_ok && suite_signal != "none" {
        "repair_pending"
    } else if check_ok || suite_ok || arcadia_ok {
        "current"
    } else if check.is_none() && suite.is_none() {
        "unknown"
    } else {
        "repair_pending"
    };
    let modules = harmonia_module_statuses(&profile);
    let last_update_run = harmonia_last_run_label(suite_receipt, check_receipt);
    let pending_updates = harmonia_pending_updates(
        &modules,
        check.as_ref(),
        suite.as_ref(),
        check_ok,
        suite_ok,
    );
    UpdatesStatus {
        state: state.to_string(),
        current_version: current,
        available_version: None,
        profile_id,
        identity,
        suite_ok,
        suite_changed,
        check_ok,
        check_changed,
        check_missing_signal,
        first_missing_signal,
        module_count,
        operation_count,
        last_update_run,
        pending_updates,
        latest_receipt: suite_receipt.to_string(),
        latest_check_receipt: check_receipt.to_string(),
        module_root: format!("{}/modules", HOMECONSOLE_PROFILE.trim_end_matches("/index.json")),
        modules,
    }
}

fn file_mtime_unix(path: &str) -> Option<u64> {
    fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs())
}

fn harmonia_last_run_label(suite_path: &str, check_path: &str) -> String {
    let latest = [file_mtime_unix(suite_path), file_mtime_unix(check_path)]
        .into_iter()
        .flatten()
        .max();
    latest
        .map(format_relative_age)
        .unwrap_or_else(|| "Never".to_string())
}

fn format_relative_age(run_unix: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(run_unix);
    let age = now.saturating_sub(run_unix);
    if age < 60 {
        "Just now".to_string()
    } else if age < 3_600 {
        format!("{}m ago", age / 60)
    } else if age < 86_400 {
        format!("{}h ago", age / 3_600)
    } else {
        format!("{}d ago", age / 86_400)
    }
}

fn harmonia_pending_updates(
    modules: &[HarmoniaModuleStatus],
    check: Option<&serde_json::Value>,
    suite: Option<&serde_json::Value>,
    check_ok: bool,
    suite_ok: bool,
) -> usize {
    let enabled = modules.iter().filter(|module| module.enabled).count();
    let ready = modules
        .iter()
        .filter(|module| module.enabled && module.present)
        .count();
    let missing = enabled.saturating_sub(ready);
    if let Some(check) = check {
        if !check_ok {
            return receipt_usize(check, "operation_count")
                .unwrap_or(0)
                .max(missing);
        }
    }
    if let Some(suite) = suite {
        if !suite_ok {
            return receipt_usize(suite, "operation_count")
                .unwrap_or(0)
                .max(missing);
        }
    }
    missing
}

fn read_json_value(path: &str) -> Option<serde_json::Value> {
    fs::read_to_string(path).ok().and_then(|text| serde_json::from_str(&text).ok())
}

fn receipt_string(value: &serde_json::Value, key: &str) -> Option<String> {
    value.get(key).and_then(|v| v.as_str()).map(str::to_string)
}

fn receipt_bool(value: &serde_json::Value, key: &str) -> Option<bool> {
    value.get(key).and_then(|v| v.as_bool())
}

fn receipt_usize(value: &serde_json::Value, key: &str) -> Option<usize> {
    value.get(key).and_then(|v| v.as_u64()).map(|v| v as usize)
}

fn harmonia_profile_modules() -> Vec<String> {
    read_json_value(HOMECONSOLE_PROFILE)
        .and_then(|json| {
            json.get("modules")
                .and_then(|modules| modules.as_array())
                .map(|modules| modules.iter().filter_map(|m| m.as_str().map(str::to_string)).collect())
        })
        .filter(|modules: &Vec<String>| !modules.is_empty())
        .unwrap_or_else(|| {
            vec![
                "identity",
                "system-packages",
                "harmonia-runtime",
                "keyman-runtime",
                "homeconsole-sync-runtime",
                "rust-build-toolchain",
                "arcadia-gui-runtime",
                "pinned-artifacts-runtime",
            ]
            .into_iter()
            .map(str::to_string)
            .collect()
        })
}

fn harmonia_all_known_modules(enabled: &[String]) -> Vec<String> {
    let mut modules = enabled.to_vec();
    let module_root = Path::new(HOMECONSOLE_PROFILE).parent().unwrap_or_else(|| Path::new("/etc/harmonia/profiles/homeconsole")).join("modules");
    if let Ok(entries) = fs::read_dir(module_root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if !modules.iter().any(|module| module == name) {
                        modules.push(name.to_string());
                    }
                }
            }
        }
    }
    modules
}

fn harmonia_module_statuses(enabled: &[String]) -> Vec<HarmoniaModuleStatus> {
    let all = harmonia_all_known_modules(enabled);
    let module_root = Path::new(HOMECONSOLE_PROFILE).parent().unwrap_or_else(|| Path::new("/etc/harmonia/profiles/homeconsole")).join("modules");
    all.into_iter()
        .map(|id| {
            let enabled_flag = enabled.iter().any(|module| module == &id);
            let present = module_root.join(&id).exists();
            let receipt_path = format!("/var/lib/harmonia/receipts/homeconsole-latest/modules/{}/run.json", id);
            let state = if !enabled_flag {
                "disabled"
            } else if present {
                "enabled"
            } else {
                "missing"
            };
            HarmoniaModuleStatus {
                label: harmonia_module_label(&id),
                id,
                enabled: enabled_flag,
                present,
                state: state.to_string(),
                receipt_path,
            }
        })
        .collect()
}

fn harmonia_module_label(id: &str) -> String {
    id.split('-')
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
