fn home_view(status: &ConsoleStatus) -> Markup {
    html! {
        section id="view-home" class="view" data-view-panel="home" tabindex="-1" {
            (priority_strip(status))
            div class="home-operational-grid home-operational-grid--dashboard" {
                (home_storage_card(status))
                (home_load_card())
                (home_sync_card(status))
                (home_network_card(status))
                (home_updates_card(status))
                (home_local_ai_card(status))
                (home_system_health_card(status))
                (home_identity_card(status))
                @if status.arcadia.service != "running" {
                    (home_gamescope_card(status))
                }
            }
            @if status.library.last_sync_state == "error" || status.local_ai.load_state == "error" {
                div class="active-warning-strip home-warning-strip" {
                    @if status.library.last_sync_state == "error" {
                        strong { "Sync failed" }
                    } @else {
                        strong { "Local AI error" }
                    }
                }
            }
        }
    }
}

fn priority_strip(status: &ConsoleStatus) -> Markup {
    let priority: Option<(String, String, &'static str)> = if !status.network.online {
        Some((
            "Network offline".to_string(),
            "Console network is unavailable; network controls stay in the left pane.".to_string(),
            "bad",
        ))
    } else if status.storage.percent_used >= 90 {
        Some((
            format!("Storage low: {} free", status.storage.free),
            format!("{} used across managed storage.", status.storage.percent),
            "warn",
        ))
    } else if status.library.last_sync_state == "error" {
        Some((
            "Sync failed".to_string(),
            "Latest receipt reports a game-library sync failure.".to_string(),
            "bad",
        ))
    } else if !sync_has_history(status) {
        Some((
            "First sync waiting".to_string(),
            format!(
                "{} playable ROMs are visible before the first verified sync receipt.",
                status.library.total_detected_games
            ),
            "idle",
        ))
    } else if status.library.sync_needed {
        let changes = status.library.unsynced_added
            + status.library.unsynced_changed
            + status.library.unsynced_removed;
        Some((
            format!("{} changes waiting for sync", changes),
            "ROM folder changes are queued for the Sync pane.".to_string(),
            "warn",
        ))
    } else if status.updates.state == "available" {
        Some((
            "Update available".to_string(),
            status
                .updates
                .available_version
                .clone()
                .unwrap_or_else(|| "Harmonia reports available work.".to_string()),
            "warn",
        ))
    } else if status.local_ai.load_state == "error" {
        Some((
            "Local AI error".to_string(),
            status
                .local_ai
                .selected_model_name
                .clone()
                .unwrap_or_else(|| "Model load failed.".to_string()),
            "bad",
        ))
    } else {
        None
    };

    if let Some((state, _detail, tone)) = priority {
        let badge = if tone == "idle" {
            "Waiting"
        } else {
            "Attention"
        };
        html! {
            article class=(format!("priority-strip priority-strip--{}", tone)) aria-label="Highest priority console state" {
                strong { (state) }
                b class=(format!("system-status system-status--{}", if tone == "bad" { "error" } else if tone == "warn" { "starting" } else { "unknown" })) { (badge) }
            }
        }
    } else {
        html! {}
    }
}

fn home_storage_card(status: &ConsoleStatus) -> Markup {
    let warning_count = status.storage.diagnostics.warnings.len()
        + status.storage.diagnostics.missing_dirs.len()
        + status.storage.diagnostics.permission_errors.len();
    let cleanup_bytes = status.storage.cleanup.artwork_bytes_clearable
        + status.storage.cleanup.temporary_bytes_clearable
        + status.storage.cleanup.partial_downloads_bytes_clearable
        + status.storage.cleanup.old_update_bytes_clearable
        + status.storage.cleanup.logs_bytes_clearable;
    let cleanup_label = human_size(cleanup_bytes);
    html! {
        article class=(if status.storage.percent_used >= 90 { "operational-card storage-home-card attention" } else { "operational-card storage-home-card" }) {
            div class="card-head" aria-label="Storage" { h3 { "Storage" } strong { (status.storage.free) " free" } }
            div class="storage-bar storage-bar--home" aria-label="Storage usage by category" {
                span class="storage-segment storage-segment--games" style=(format!("width: {}%", status.storage.games.percent_of_total.max(if status.storage.games.bytes > 0 { 1 } else { 0 }))) title=(format!("Games {}", status.storage.games.size)) {}
                span class="storage-segment storage-segment--artwork" style=(format!("width: {}%", status.storage.artwork.percent_of_total.max(if status.storage.artwork.bytes > 0 { 1 } else { 0 }))) title=(format!("Artwork {}", status.storage.artwork.size)) {}
                span class="storage-segment storage-segment--ai" style=(format!("width: {}%", status.storage.ai_models.percent_of_total.max(if status.storage.ai_models.bytes > 0 { 1 } else { 0 }))) title=(format!("AI Models {}", status.storage.ai_models.size)) {}
                span class="storage-segment storage-segment--other" style=(format!("width: {}%", status.storage.other.percent_of_total.max(if status.storage.other.bytes > 0 { 1 } else { 0 }))) title=(format!("Other {}", status.storage.other.size)) {}
                span class="storage-segment storage-segment--free" style=(format!("width: {}%", 100u8.saturating_sub(status.storage.percent_used))) title=(format!("Free {}", status.storage.free)) {}
            }
            div class="storage-mini-rows" {
                (storage_mini_row("Games", &status.storage.games.size, status.storage.games.percent_of_total, status.storage.games.bytes))
                (storage_mini_row("Artwork", &status.storage.artwork.size, status.storage.artwork.percent_of_total, status.storage.artwork.bytes))
                (storage_mini_row("AI Models", &status.storage.ai_models.size, status.storage.ai_models.percent_of_total, status.storage.ai_models.bytes))
                (storage_mini_row("Other", &status.storage.other.size, status.storage.other.percent_of_total, status.storage.other.bytes))
            }
            div class="home-signal-strip" aria-label="Storage signals" {
                (home_signal("Volumes", &status.storage.volumes.len().to_string(), "idle"))
                (home_signal("Cleanup", &cleanup_label, if cleanup_bytes > 0 { "warn" } else { "ok" }))
                (home_signal("Warnings", &warning_count.to_string(), if warning_count > 0 { "warn" } else { "ok" }))
            }
        }
    }
}

fn home_load_card() -> Markup {
    let load = load_average();
    let one = json_number(&load, "oneMinute");
    let five = json_number(&load, "fiveMinute");
    let fifteen = json_number(&load, "fifteenMinute");
    let cores = std::thread::available_parallelism()
        .map(|count| count.get() as f64)
        .unwrap_or(1.0)
        .max(1.0);
    let load_percent = one
        .map(|value| ((value / cores) * 100.0).clamp(0.0, 100.0).round() as u8)
        .unwrap_or(0);
    let load_state = if load_percent >= 90 {
        "warn"
    } else if one.is_some() {
        "ok"
    } else {
        "idle"
    };
    let temp = cpu_temperature_celsius();
    let temp_label = temp
        .map(|value| format!("{value:.1}°C"))
        .unwrap_or_else(|| "—".to_string());
    let temp_state = match temp {
        Some(value) if value >= 82.0 => "warn",
        Some(_) => "ok",
        None => "idle",
    };
    let io_pressure = pressure_avg10_percent("/proc/pressure/io");
    let io_label = io_pressure
        .map(|value| format!("{value:.1}%"))
        .unwrap_or_else(|| "—".to_string());
    let io_state = match io_pressure {
        Some(value) if value >= 10.0 => "warn",
        Some(_) => "ok",
        None => "idle",
    };
    let disk = disk_io_counters();
    let read_bytes = json_u64(&disk, "readBytesApprox");
    let written_bytes = json_u64(&disk, "writtenBytesApprox");
    let load_headline = one
        .map(|value| format!("{value:.2}"))
        .unwrap_or_else(|| "—".to_string());
    html! {
        article class="operational-card load-home-card" aria-label="Load dashboard" data-load-card data-load-retry-ms="5000" {
            div class="card-head" aria-label="Load" { h3 { "Load" } strong data-load-headline { (load_headline) } }
            div class="load-orb-row" {
                div class=(format!("load-orb load-orb--{}", load_state)) style=(format!("--load-pct:{};", load_percent)) aria-label=(format!("{} percent load", load_percent)) data-load-orb {
                    span data-load-percent { (load_percent) "%" }
                }
                div class="load-spark-bank" aria-label="Load average" {
                    (load_spark("1m", "oneMinute", one, cores))
                    (load_spark("5m", "fiveMinute", five, cores))
                    (load_spark("15m", "fifteenMinute", fifteen, cores))
                }
            }
            div class="load-telemetry-grid" aria-label="Telemetry" {
                (load_chip("CPU", "cpu", &temp_label, temp_state))
                (load_chip("I/O", "io", &io_label, io_state))
                (load_chip("Read", "read", &read_bytes.map(human_size).unwrap_or_else(|| "—".to_string()), "idle"))
                (load_chip("Write", "write", &written_bytes.map(human_size).unwrap_or_else(|| "—".to_string()), "idle"))
            }
        }
    }
}

fn load_spark(label: &str, key: &str, value: Option<f64>, cores: f64) -> Markup {
    let width = value
        .map(|number| ((number / cores) * 100.0).clamp(0.0, 100.0).round() as u8)
        .unwrap_or(0);
    let display = value
        .map(|number| format!("{number:.2}"))
        .unwrap_or_else(|| "—".to_string());
    html! {
        div class="load-spark" data-load-spark=(key) {
            span { (label) }
            i { em style=(format!("width:{}%;", width)) data-load-spark-bar=(key) {} }
            strong data-load-spark-value=(key) { (display) }
        }
    }
}

fn load_chip(label: &str, key: &str, value: &str, state: &str) -> Markup {
    html! {
        div class=(format!("load-chip load-chip--{}", state)) data-load-chip=(key) {
            em { (label) }
            strong data-load-chip-value=(key) { (value) }
        }
    }
}

fn json_number(value: &serde_json::Value, key: &str) -> Option<f64> {
    value.get(key).and_then(serde_json::Value::as_f64)
}

fn json_u64(value: &serde_json::Value, key: &str) -> Option<u64> {
    value.get(key).and_then(serde_json::Value::as_u64)
}

fn home_network_card(status: &ConsoleStatus) -> Markup {
    let internet_label = match status.network.internet_reachable {
        Some(true) => "Online".to_string(),
        Some(false) => "No internet".to_string(),
        None => "Unchecked".to_string(),
    };
    html! {
        article class=(if status.network.online { "operational-card network-home-card" } else { "operational-card network-home-card attention" }) {
            @if status.network.online {
                div class="card-head" aria-label="Network" { h3 { "Network" } strong { (status.network.ip_address) } }
                @if status.network.active_type == "wifi" {
                    div class="home-signal-strip" aria-label="Wi-Fi signal" { (home_signal("Wi-Fi", &status.network.signal_percent.map(|v| format!("{}%", v)).unwrap_or_else(|| "—".to_string()), "idle")) }
                } @else {
                    div class="home-signal-strip" aria-label="Ethernet speed" { (home_signal("Ethernet", &status.network.ethernet_speed_mbps.map(|v| format!("{} Mbps", v)).unwrap_or_else(|| "—".to_string()), "idle")) }
                }
                div class="home-topology" aria-label="Network topology" {
                    span { "Console" }
                    i { "→" }
                    span { "Home LAN" }
                    i { "→" }
                    span { (internet_label) }
                }
                div class="reachability-row reachability-row--topology" {
                    (reachability("Link", status.network.online))
                    @if let Some(internet) = status.network.internet_reachable { (reachability("Internet", internet)) }
                    (reachability("Samba", status.network.samba_reachable))
                    (reachability("AI", status.network.lan_ai_reachable))
                }
                div class="home-code-line" aria-label="Console URL" { code { (status.identity.web_origin) } }
            } @else {
                div class="card-head" aria-label="Network" { h3 { "Network" } strong { "Offline" } }
                div class="home-signal-strip" { (home_signal("Ethernet", if status.network.ethernet_available { "Present" } else { "Absent" }, if status.network.ethernet_available { "idle" } else { "warn" })) (home_signal("Wi-Fi", if status.network.wifi_adapter_available { "Present" } else { "Absent" }, if status.network.wifi_adapter_available { "idle" } else { "warn" })) }
            }
        }
    }
}

fn home_sync_card(status: &ConsoleStatus) -> Markup {
    let pending_changes = status.library.unsynced_added
        + status.library.unsynced_changed
        + status.library.unsynced_removed;
    let state = if status.library.last_sync_state == "error" {
        "Needs attention"
    } else if status.library.last_sync_state == "running" {
        "Scanning"
    } else if pending_changes > 0 || status.library.sync_needed {
        "Sync needed"
    } else if !sync_has_history(status) {
        "First sync waiting"
    } else {
        "Synced"
    };
    let sync_delta = status
        .library
        .total_detected_games
        .saturating_sub(status.library.total_synced_entries);
    html! {
        article class=(if status.library.last_sync_state == "error" || pending_changes > 0 || status.library.sync_needed { "operational-card sync-home-card attention" } else { "operational-card sync-home-card" }) data-home-sync-state=(state) {
            div class="card-head" aria-label="Game library" { h3 { "Games" } strong { (status.library.total_detected_games) " / " (status.library.total_synced_entries) } }
            div class="state-rows state-rows--compact" {
                (state_row("Available ROMs", &status.library.total_detected_games.to_string()))
                (state_row("Last scan", &status.library.last_sync))
                (state_row("Folder changes", &pending_changes.to_string()))
                (state_row("Library gap", &sync_delta.to_string()))
                (state_row("Artwork", &status.library.artwork_status))
            }
        }
    }
}

fn home_local_ai_card(status: &ConsoleStatus) -> Markup {
    let model_line = status
        .local_ai
        .loaded_model_name
        .as_deref()
        .or(status.local_ai.selected_model_name.as_deref())
        .unwrap_or(if status.local_ai.available_models.is_empty() {
            "No models installed"
        } else {
            "No model loaded"
        });
    let accelerator = status.local_ai.gpu_memory.as_deref();
    let lan = if let Some(port) = status.local_ai.lan_inference_port {
        format!("On · :{}", port)
    } else {
        "Off".to_string()
    };
    html! {
        article class=(if status.local_ai.load_state == "error" { "operational-card local-ai-home-card attention" } else { "operational-card local-ai-home-card" }) {
            div class="card-head" aria-label="Local AI" { h3 { "Local AI" } strong { (model_line) } }
            div class="state-rows state-rows--compact" {
                @if let Some(accelerator) = accelerator { (state_row("Accelerator", accelerator)) }
                (state_row("LAN", &lan))
                (state_row("Models", &status.local_ai.available_models.len().to_string()))
            }
            @if let (Some(used), Some(total)) = (status.local_ai.gpu_memory_used_bytes, status.local_ai.gpu_memory_total_bytes) {
                div class="gpu-bar" aria-label="Accelerator memory usage" { span style=(format!("width: {}%", ((used.saturating_mul(100) / total.max(1)).min(100)))) {} }
            }
        }
    }
}

fn home_updates_card(status: &ConsoleStatus) -> Markup {
    let (ready, enabled) = harmonia_module_readiness(&status.updates.modules);
    let ratio = format!("{ready}/{enabled}");
    let available_line = status.updates.pending_updates.to_string();
    let needs_attention = status.updates.pending_updates > 0
        || ready < enabled
        || !status.updates.check_ok && status.updates.check_missing_signal != "not-checked"
        || !status.updates.suite_ok;
    html! {
        article class=(if needs_attention { "operational-card updates-home-card attention" } else { "operational-card updates-home-card" }) data-home-harmonia-state=(&status.updates.state) {
            div class="card-head" aria-label="Updates" {
                h3 { "Updates" }
                strong { (ratio) }
            }
            div class="state-rows state-rows--compact updates-home-details" {
                (updates_detail_row("Last ran:", &status.updates.last_update_run, false))
                (updates_detail_row("updates available:", &available_line, true))
            }
            div class="inline-actions inline-actions--compact updates-home-actions" {
                (action_button(ButtonVariant::Primary, "Check", "check-updates", "/api/actions/check-updates"))
            }
        }
    }
}

fn updates_detail_row(label: &str, value: &str, inline_count: bool) -> Markup {
    let row_class = if inline_count {
        "state-row updates-detail-row updates-detail-row--count"
    } else {
        "state-row updates-detail-row"
    };
    html! {
        div class=(row_class) aria-label=(format!("{label} {value}")) {
            span { (label) }
            strong { (value) }
        }
    }
}

fn home_system_health_card(status: &ConsoleStatus) -> Markup {
    let service_total = status.system.services.len();
    let running = status
        .system
        .services
        .iter()
        .filter(|svc| matches!(svc.state.as_str(), "running" | "available" | "enabled"))
        .count();
    let failed = service_total.saturating_sub(running);
    html! {
        article class=(if failed > 0 { "operational-card health-home-card attention" } else { "operational-card health-home-card" }) {
            div class="card-head" aria-label="System Health" { h3 { "Health" } strong { (running) "/" (service_total) } }
            div class="home-service-list" aria-label="Appliance services" {
                @for svc in status.system.services.iter().take(2) {
                    div class="home-service-row" data-label=(&svc.name) aria-label=(format!("{} {}", svc.name, title_case_state_like(&svc.state))) {
                        b class=(status_class(&svc.state)) data-state=(&svc.state) { (if matches!(svc.state.as_str(), "running" | "available" | "enabled") { "✓" } else { "!" }) }
                    }
                }
            }
        }
    }
}

fn home_identity_card(status: &ConsoleStatus) -> Markup {
    let lock_state = if status.gui_pin.pin_required {
        "PIN required"
    } else {
        "Open"
    };
    let trust_state = if status.system.trust.mode == "https" {
        "Secure web"
    } else {
        "Local HTTP"
    };
    html! {
        article class="operational-card identity-home-card" {
            div class="card-head" aria-label="Appliance" { h3 { "Appliance" } strong { (&status.identity.hostname) } }
            div class="state-rows state-rows--compact" {
                (state_row("Address", &status.network.ip_address))
                @if status.gui_pin.pin_required { (state_row("Access", lock_state)) }
                @if status.system.trust.mode == "https" { (state_row("Trust", trust_state)) }
            }
        }
    }
}

fn home_gamescope_card(status: &ConsoleStatus) -> Markup {
    html! {
        article class="operational-card gamescope-home-card attention" {
            div class="card-head" aria-label="Game Session" { h3 { "Session" } strong data-state=(status.arcadia.service) { "!" } }
            div class="home-signal-strip" { (home_signal("Interface", status.arcadia.service, if status.arcadia.service == "running" { "ok" } else { "warn" })) }
        }
    }
}

fn home_signal(label: &str, value: &str, tone: &str) -> Markup {
    html! { span class=(format!("home-signal home-signal--{}", tone)) data-label=(label) aria-label=(format!("{} {}", label, value)) { strong { (value) } } }
}

fn title_case_state_like(state: &str) -> &'static str {
    match state {
        "unloaded" => "Unloaded",
        "cold" => "Cold",
        "loading" => "Loading",
        "hot" => "Hot",
        "error" => "Error",
        "running" => "Running",
        "available" => "Available",
        "enabled" => "Enabled",
        "stopped" => "Stopped",
        "disabled" => "Disabled",
        "success" => "Success",
        "current" => "Current",
        "repair_pending" => "Repair pending",
        "ready to save" => "Ready to save",
        "saved" => "Saved",
        "waiting for controller" => "Waiting",
        "waiting" => "Waiting",
        "ready" => "Ready",
        "listening" => "Listening",
        "active" => "Active",
        "configured" => "Configured",
        "needs setup" => "Needs setup",
        "not installed" => "Not installed",
        "connected" => "Connected",
        "receiver-only" => "Receiver only",
        "disconnected" => "Disconnected",
        _ => "Unknown",
    }
}

fn storage_mini_row(label: &str, value: &str, percent: u8, bytes: u64) -> Markup {
    html! { div class="storage-mini-row" data-label=(label) aria-label=(format!("{} {} {}", label, value, percent_label(percent, bytes))) { strong { (value) } em { (percent_label(percent, bytes)) } } }
}

fn percent_label(percent: u8, bytes: u64) -> String {
    if bytes > 0 && percent == 0 {
        "<1%".to_string()
    } else {
        format!("{}%", percent)
    }
}

fn state_row(label: &str, value: &str) -> Markup {
    html! { div class="state-row" data-label=(label) aria-label=(format!("{} {}", label, value)) { strong { (value) } } }
}

fn reachability(label: &str, ok: bool) -> Markup {
    html! { span class=(if ok { "reachability reachability--ok" } else { "reachability" }) data-label=(label) aria-label=(format!("{} {}", label, if ok { "ok" } else { "unavailable" })) { (if ok { "✓" } else { "—" }) } }
}

