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
    let everything_else = home_storage_everything_else_size(status);
    html! {
        article class=(if status.storage.percent_used >= 90 { "operational-card storage-home-card attention" } else { "operational-card storage-home-card" }) {
            div class="card-head" aria-label="Storage" {
                h3 { "Storage" }
            }
            div class="storage-bar storage-bar--home" aria-label="Storage usage by category" {
                span class="storage-segment storage-segment--games" style=(format!("width: {}%", status.storage.games.percent_of_total.max(if status.storage.games.bytes > 0 { 1 } else { 0 }))) title=(format!("Games {}", status.storage.games.size)) {}
                span class="storage-segment storage-segment--artwork" style=(format!("width: {}%", status.storage.artwork.percent_of_total.max(if status.storage.artwork.bytes > 0 { 1 } else { 0 }))) title=(format!("Artwork {}", status.storage.artwork.size)) {}
                span class="storage-segment storage-segment--ai" style=(format!("width: {}%", status.storage.ai_models.percent_of_total.max(if status.storage.ai_models.bytes > 0 { 1 } else { 0 }))) title=(format!("AI Models {}", status.storage.ai_models.size)) {}
                span class="storage-segment storage-segment--other" style=(format!("width: {}%", status.storage.other.percent_of_total.max(if status.storage.other.bytes > 0 { 1 } else { 0 }))) title=(format!("Other {}", status.storage.other.size)) {}
                span class="storage-segment storage-segment--free" style=(format!("width: {}%", 100u8.saturating_sub(status.storage.percent_used))) title=(format!("Free {}", status.storage.free)) {}
            }
            div class="state-rows state-rows--compact storage-home-details" {
                (home_detail_row("Games used:", &status.storage.games.size, true))
                (home_detail_row("AI used:", &status.storage.ai_models.size, true))
                (home_detail_row("Everything else:", &everything_else, true))
                (home_detail_row("Free:", &status.storage.free, true))
            }
        }
    }
}

fn home_storage_everything_else_size(status: &ConsoleStatus) -> String {
    human_size(
        status
            .storage
            .artwork
            .bytes
            .saturating_add(status.storage.other.bytes),
    )
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
    let read_rate = json_u64(&disk, "readBytesPerSec");
    let write_rate = json_u64(&disk, "writeBytesPerSec");
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
                (load_chip("Read/s", "read", &read_rate.map(human_rate).unwrap_or_else(|| "—".to_string()), if read_rate.unwrap_or(0) > 0 { "ok" } else { "idle" }))
                (load_chip("Write/s", "write", &write_rate.map(human_rate).unwrap_or_else(|| "—".to_string()), if write_rate.unwrap_or(0) > 0 { "ok" } else { "idle" }))
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

fn human_rate(bytes_per_sec: u64) -> String {
    if bytes_per_sec == 0 {
        return "0 B/s".to_string();
    }
    if bytes_per_sec < 1024 {
        return format!("{bytes_per_sec} B/s");
    }
    if bytes_per_sec < 1024 * 1024 {
        return format!("{} KB/s", bytes_per_sec / 1024);
    }
    let mb = bytes_per_sec as f64 / 1024.0 / 1024.0;
    if mb >= 10.0 {
        format!("{mb:.0} MB/s")
    } else {
        format!("{mb:.1} MB/s")
    }
}

fn home_network_card(status: &ConsoleStatus) -> Markup {
    html! {
        article class=(if status.network.online { "operational-card network-home-card" } else { "operational-card network-home-card attention" }) {
            div class="card-head" aria-label="Network" {
                h3 { "Network" }
                strong { (if status.network.online { status.network.ip_address.as_str() } else { "Offline" }) }
            }
            (home_network_stack(status))
            @if status.network.online {
                div class="home-code-line" aria-label="Console URL" { code { (status.identity.web_origin) } }
            }
        }
    }
}

fn home_network_stack(status: &ConsoleStatus) -> Markup {
    let modem_ok = status.network.internet_reachable == Some(true);
    let modem_state = match status.network.internet_reachable {
        Some(true) => "Online",
        Some(false) => "Offline",
        None => "Unknown",
    };
    let lan_ok = status.network.online
        && !status.network.ip_address.is_empty()
        && status.network.ip_address != "Unknown";
    let lan_state = if lan_ok {
        status.network.ip_address.as_str()
    } else {
        "Offline"
    };
    let us_ok = status.network.online;
    let us_state = if us_ok {
        match status.network.internet_reachable {
            Some(true) => "Online",
            Some(false) => "LAN only",
            None => "On LAN",
        }
    } else {
        "Offline"
    };
    html! {
        div class="home-network-stack" aria-label="Network path from modem to console" {
            (home_network_node("Modem", modem_state, modem_ok))
            div class="home-network-link" aria-hidden="true" { "│" }
            (home_network_node("Home LAN", lan_state, lan_ok))
            div class="home-network-link" aria-hidden="true" { "│" }
            (home_network_node(&status.identity.hostname, us_state, us_ok))
        }
    }
}

fn home_network_node(label: &str, state: &str, ok: bool) -> Markup {
    html! {
        div class=(format!("home-network-node home-network-node--{}", if ok { "ok" } else { "warn" })) aria-label=(format!("{label} {state}")) {
            strong class="home-network-node__label" { (label) }
            span class="home-network-node__state" { (state) }
        }
    }
}

fn home_sync_card(status: &ConsoleStatus) -> Markup {
    let pending_changes = status.library.unsynced_added
        + status.library.unsynced_changed
        + status.library.unsynced_removed;
    let native = status.library.gamescope_entries;
    let added = status.library.total_detected_games;
    let native_line = native.to_string();
    let added_line = added.to_string();
    let total_line = library_games_total(status).to_string();
    let needs_attention = status.library.last_sync_state == "error"
        || pending_changes > 0
        || status.library.sync_needed;
    html! {
        article class=(if needs_attention { "operational-card sync-home-card attention" } else { "operational-card sync-home-card" }) data-home-games-total=(total_line) {
            div class="card-head" aria-label="Games" {
                h3 { "Games" }
                strong { (total_line) }
            }
            div class="state-rows state-rows--compact sync-home-details" {
                (home_detail_row("GameScope:", &native_line, true))
                (home_detail_row("Added:", &added_line, true))
            }
        }
    }
}

fn home_local_ai_card(status: &ConsoleStatus) -> Markup {
    let model_name = home_local_ai_model_name(status);
    let load_label = home_local_ai_load_label(&status.local_ai.load_state);
    let activity_label = home_local_ai_activity_label(status);
    html! {
        article class=(if status.local_ai.load_state == "error" { "operational-card local-ai-home-card attention" } else { "operational-card local-ai-home-card" }) data-home-ai-load-state=(&status.local_ai.load_state) {
            div class="card-head" aria-label="AI Model" {
                h3 { "AI Model" }
            }
            div class="state-rows state-rows--compact local-ai-home-details" {
                (home_detail_row("Model:", &model_name, false))
                (home_detail_row("Load:", load_label, true))
                (home_detail_row("State:", activity_label, true))
            }
        }
    }
}

fn home_local_ai_model_name(status: &ConsoleStatus) -> String {
    status
        .local_ai
        .loaded_model_name
        .clone()
        .or_else(|| status.local_ai.selected_model_name.clone())
        .unwrap_or_else(|| {
            if status.local_ai.available_models.is_empty() {
                "No models installed".to_string()
            } else {
                "No model selected".to_string()
            }
        })
}

fn home_local_ai_load_label(load_state: &str) -> &'static str {
    match load_state {
        "hot" => "Hot",
        "cold" => "Cold",
        "unloaded" => "Unloaded",
        "error" => "Error",
        _ => "Unknown",
    }
}

fn home_local_ai_activity_label(status: &ConsoleStatus) -> &'static str {
    if status.local_ai.load_state == "error" {
        return "Error";
    }
    if status.local_ai.load_state == "hot" && status.local_ai.lan_inference_enabled {
        return "Actively working";
    }
    match status.local_ai.load_state.as_str() {
        "hot" => "Idle",
        "cold" => "Idle",
        "unloaded" if status.local_ai.available_models.is_empty() => "Idle",
        "unloaded" => "Not loaded",
        _ => "Idle",
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
                (home_detail_row("Last ran:", &status.updates.last_update_run, false))
                (home_detail_row("updates available:", &available_line, true))
            }
            div class="inline-actions inline-actions--compact updates-home-actions" {
                (action_button(ButtonVariant::Primary, "Check", "check-updates", "/api/actions/check-updates"))
            }
        }
    }
}

fn home_detail_row(label: &str, value: &str, inline_count: bool) -> Markup {
    let row_class = if inline_count {
        "state-row home-detail-row home-detail-row--inline"
    } else {
        "state-row home-detail-row"
    };
    html! {
        div class=(row_class) aria-label=(format!("{label} {value}")) {
            span { (label) }
            strong { (value) }
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



