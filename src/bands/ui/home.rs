fn home_view(status: &ConsoleStatus) -> Markup {
    html! {
        section id="view-home" class="view" data-view-panel="home" tabindex="-1" {
            (priority_strip(status))
            div class="home-operational-grid home-operational-grid--dashboard" {
                (home_storage_card(status))
                (home_load_card())
                (home_network_card(status))
                (home_updates_card(status))
                (home_local_ai_card(status))
            }
            div class="active-warning-strip home-warning-strip" data-bind-show="home.warning.visible" hidden[!(status.library.last_sync_state == "error" || status.local_ai.load_state == "error")] {
                strong data-bind="home.warning.title" {
                    @if status.library.last_sync_state == "error" {
                        "Sync failed"
                    } @else {
                        "Local AI error"
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

    let (state, _detail, tone) = priority.unwrap_or_else(|| (String::new(), String::new(), "idle"));
    let badge = if tone == "idle" {
        "Waiting"
    } else {
        "Attention"
    };
    let system_tone = if tone == "bad" { "error" } else if tone == "warn" { "starting" } else { "unknown" };
    html! {
        article class=(format!("priority-strip priority-strip--{}", tone)) aria-label="Highest priority console state" data-bind-show="home.priority.visible" data-bind-class="home.priority.tone" hidden[state.is_empty()] {
            strong data-bind="home.priority.state" { (state) }
            b class=(format!("system-status system-status--{}", system_tone)) data-bind="home.priority.badge" data-bind-class="home.priority.tone" { (badge) }
        }
    }
}

fn home_storage_card(status: &ConsoleStatus) -> Markup {
    let everything_else = home_storage_everything_else_size(status);
    let everything_else_bytes = status.storage.artwork.bytes.saturating_add(status.storage.other.bytes);
    let everything_else_percent = home_storage_percent_of_total(everything_else_bytes, status.storage.total_bytes);
    html! {
        article class=(if status.storage.percent_used >= 90 { "operational-card storage-home-card attention" } else { "operational-card storage-home-card" }) data-bind-class="home.storage.state" {
            div class="card-head" aria-label="Storage" {
                h3 { "Storage" }
                strong data-bind="home.storage.percentUsed" { (format!("{}% used", status.storage.percent_used)) }
            }
            div class="storage-bar storage-bar--home" aria-label="Storage usage by category" {
                span class="storage-segment storage-segment--games" style=(format!("width: {}%", status.storage.games.percent_of_total.max(if status.storage.games.bytes > 0 { 1 } else { 0 }))) title=(format!("Games {}", status.storage.games.size)) {}
                span class="storage-segment storage-segment--ai" style=(format!("width: {}%", status.storage.ai_models.percent_of_total.max(if status.storage.ai_models.bytes > 0 { 1 } else { 0 }))) title=(format!("AI Models {}", status.storage.ai_models.size)) {}
                span class="storage-segment storage-segment--other" style=(format!("width: {}%", everything_else_percent.max(if everything_else_bytes > 0 { 1 } else { 0 }))) title=(format!("Everything else {}", everything_else)) {}
                span class="storage-segment storage-segment--free" style=(format!("width: {}%", 100u8.saturating_sub(status.storage.percent_used))) title=(format!("Free {}", status.storage.free)) {}
            }
            div class="state-rows state-rows--compact storage-home-details" {
                (home_bound_detail_row_with_class("Games used:", &status.storage.games.size, true, "home.storage.gamesSize", "home-detail-row--cat-games"))
                (home_bound_detail_row_with_class("AI used:", &status.storage.ai_models.size, true, "home.storage.aiSize", "home-detail-row--cat-ai"))
                (home_bound_detail_row_with_class("Everything else:", &everything_else, true, "home.storage.everythingElseSize", "home-detail-row--cat-else"))
                (home_bound_detail_row_with_class("Free:", &status.storage.free, true, "home.storage.freeSize", "home-detail-row--cat-free"))
            }
        }
    }
}

fn home_storage_percent_of_total(bytes: u64, total_bytes: u64) -> u8 {
    if bytes == 0 || total_bytes == 0 {
        0
    } else {
        ((bytes as u128).saturating_mul(100) / total_bytes as u128).min(100) as u8
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
    let telemetry = crate::api_telemetry_data();
    let formatted = crate::api_home_telemetry_data();
    let load = telemetry.get("load").cloned().unwrap_or_default();
    let memory = telemetry.get("memory").cloned().unwrap_or_default();
    let io = telemetry.get("io").and_then(|value| value.get("disk")).cloned().unwrap_or_default();
    let one = load.get("oneMinute").and_then(serde_json::Value::as_f64);
    let cores = std::thread::available_parallelism().map(|count| count.get() as f64).unwrap_or(1.0).max(1.0);
    let five = load.get("fiveMinute").and_then(serde_json::Value::as_f64);
    let fifteen = load.get("fifteenMinute").and_then(serde_json::Value::as_f64);
    let memory_used = memory.get("usedBytes").and_then(serde_json::Value::as_u64).unwrap_or(0);
    let memory_total = memory.get("totalBytes").and_then(serde_json::Value::as_u64).unwrap_or(0);
    let memory_percent = memory.get("usedPercent").and_then(serde_json::Value::as_f64).unwrap_or(0.0).clamp(0.0, 100.0);
    let rate = |key: &str| io.get(key).and_then(serde_json::Value::as_f64).map(crate::telemetry_human_rate).unwrap_or_else(|| "—".to_string());
    let display = |path: &[&str]| -> String {
        path.iter()
            .fold(&formatted, |value, key| value.get(*key).unwrap_or(&serde_json::Value::Null))
            .as_str()
            .unwrap_or("—")
            .to_string()
    };
    let first_fan = formatted
        .get("fans")
        .and_then(serde_json::Value::as_array)
        .and_then(|fans| fans.first())
        .cloned()
        .unwrap_or_default();
    html! {
        article class="operational-card load-home-card" aria-label="Load dashboard" data-load-card data-load-retry-ms="5000" {
            div class="card-head" aria-label="Load" { h3 { "Load" } }
            div class="load-chart-wrap" aria-label="Load history" role="img" { (home_load_sparkline(&telemetry)) }
            div class="load-average-readouts" aria-label="Load average" {
                (load_readout("1m", "oneMinute", one, cores))
                (load_readout("5m", "fiveMinute", five, cores))
                (load_readout("15m", "fifteenMinute", fifteen, cores))
            }
            div class="memory-usage" aria-label="System RAM used" {
                div class="storage-bar storage-bar--home" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow=(format!("{memory_percent:.0}")) data-memory-bar {
                    span class="storage-segment storage-segment--other" style=(format!("width: {memory_percent:.0}%")) data-memory-used-segment {}
                    span class="storage-segment storage-segment--free" style=(format!("width: {:.0}%", 100.0 - memory_percent)) data-memory-free-segment {}
                }
                div class="state-rows state-rows--compact" { div class="home-detail-row" { span { "RAM used:" } strong data-memory-used data-bind="home.telemetry.memory.usedBytes" { (human_size(memory_used)) } span data-memory-total { " / " (human_size(memory_total)) } } }
            }
            div class="load-telemetry-grid" aria-label="Telemetry" {
                (load_chip_bound("CPU temp", "cpu", &display(&["cpu", "temperatureCelsius"]), "home.telemetry.cpu.temperatureCelsius"))
                (load_chip_bound("CPU usage", "cpu-usage", &display(&["cpu", "usagePercent"]), "home.telemetry.cpu.usagePercent"))
                (load_chip_bound("I/O", "io", &display(&["io", "pressureAvg10"]), "home.telemetry.io.pressureAvg10"))
                (load_chip_bound("GPU", "gpu", &display(&["gpu", "utilizationPercent"]), "home.telemetry.gpu.utilizationPercent"))
                (load_chip_bound("GPU temp", "gpu-temperature", &display(&["gpu", "temperatureCelsius"]), "home.telemetry.gpu.temperatureCelsius"))
                (load_chip_bound("Storage temp", "storage-temperature", &display(&["temperature", "storage"]), "home.telemetry.temperature.storage"))
                (load_chip_bound("Fan RPM", "fan", first_fan.get("rpm").and_then(serde_json::Value::as_str).unwrap_or("—"), "home.telemetry.fans.0.rpm"))
                (load_chip_bound("Read/s", "read", &rate("readBytesPerSec"), "home.telemetry.io.disk.readBytesPerSec"))
                (load_chip_bound("Write/s", "write", &rate("writeBytesPerSec"), "home.telemetry.io.disk.writeBytesPerSec"))
            }
        }
    }
}

fn home_load_sparkline(telemetry: &serde_json::Value) -> Markup {
    let values: Vec<f64> = telemetry.get("history").and_then(|h| h.get("tiers")).and_then(|t| t.get("minute")).and_then(serde_json::Value::as_array).into_iter().flatten().filter_map(|entry| entry.get("aggregation").and_then(|a| a.get("loadOne")).and_then(serde_json::Value::as_f64)).collect();
    if values.is_empty() {
        return html! { span class="load-sparkline-placeholder" { "Gathering history…" } };
    }
    let max = values.iter().copied().fold(1.0, f64::max);
    let denominator = values.len().saturating_sub(1).max(1) as f64;
    let points = values.iter().enumerate().map(|(i, value)| format!("{:.1},{:.1}", if values.len() == 1 { 50.0 } else { i as f64 * 100.0 / denominator }, 38.0 - value / max * 34.0)).collect::<Vec<_>>().join(" ");
    html! { svg class="load-sparkline" viewBox="0 0 100 40" preserveAspectRatio="none" role="img" { title { "Minute load history" } desc { "Load average over the most recent minute buckets." } polyline points=(points) fill="none" stroke="currentColor" stroke-width="1.5" {} } }
}

fn load_readout(label: &str, key: &str, value: Option<f64>, cores: f64) -> Markup {
    let display = value
        .map(|number| ((number / cores) * 100.0).clamp(0.0, 100.0))
        .map(|percent| format!("{percent:.1}%"))
        .unwrap_or_else(|| "—".to_string());
    html! {
        div class="load-average-readout" data-load-readout=(key) {
            span { (label) }
            strong data-load-readout-value=(key) { (display) }
        }
    }
}

fn load_chip_bound(label: &str, key: &str, value: &str, bind: &str) -> Markup {
    html! { div class="load-chip load-chip--idle" data-load-chip=(key) { em { (label) } strong data-load-chip-value=(key) data-bind=(bind) { (value) } } }
}

fn home_network_card(status: &ConsoleStatus) -> Markup {
    let console_url = status.identity.web_origin.as_str();
    let ip_copyable = home_network_ip_copyable(&status.network.ip_address);
    let ai_copy = home_network_ai_copy_value(status);
    let headline = if status.network.online { "Online" } else { "Offline" };
    html! {
        article class=(if status.network.online { "operational-card network-home-card" } else { "operational-card network-home-card attention" }) data-bind-class="home.network.state" {
            div class="card-head" aria-label="Network" {
                h3 { "Network" }
                strong data-bind="home.network.headline" { (headline) }
            }
            div class="home-network-chip-row" aria-label="Network reachability" {
                (home_network_chip("Console", status.network.online))
                (home_network_chip("AI", status.network.lan_ai_reachable))
                (home_network_chip(
                    "Internet",
                    matches!(status.network.internet_reachable, Some(true)),
                ))
            }
            div class="inline-actions inline-actions--compact home-network-copy-row" aria-label="Network copy actions" {
                (copy_button("Copy URL", console_url))
                @if ip_copyable {
                    (copy_button("Copy IP", &status.network.ip_address))
                } @else {
                    button class="btn btn--secondary" type="button" disabled title="No LAN IP is available yet." { "Copy IP" }
                }
                @if let Some(value) = ai_copy.as_deref() {
                    (copy_button("Copy AI", value))
                } @else {
                    button class="btn btn--secondary" type="button" disabled title="LAN AI is not reachable on the saved port." { "Copy AI" }
                }
            }
        }
    }
}

fn home_network_ip_copyable(ip: &str) -> bool {
    !ip.is_empty() && ip != "—" && ip != "Unknown"
}

fn home_network_ai_copy_value(status: &ConsoleStatus) -> Option<String> {
    if !status.network.lan_ai_reachable {
        return None;
    }
    let port = status.local_ai.lan_inference_port?;
    let ip = status.network.ip_address.as_str();
    if !home_network_ip_copyable(ip) {
        return None;
    }
    Some(format!("{ip}:{port}"))
}

fn home_network_chip(label: &str, online: bool) -> Markup {
    let state = if online { "Online" } else { "Offline" };
    let tone = if online { "ok" } else { "warn" };
    let bind_path = match label {
        "Console" => "home.network.consoleReachability",
        "AI" => "home.network.aiReachability",
        "Internet" => "home.network.internetReachability",
        _ => "home.network.consoleReachability",
    };
    html! {
        span class=(format!("home-network-chip home-network-chip--{tone}")) aria-label=(format!("{label} {state}")) data-bind-class=(bind_path) {
            (label)
        }
    }
}

fn home_local_ai_card(status: &ConsoleStatus) -> Markup {
    let load_label = home_local_ai_load_label(&status.local_ai.load_state);
    let activity_label = home_local_ai_activity_label(status);
    let models = crate::api_home_ai_models(status);
    let resident_engines = &status.local_ai.resident_engines;
    html! {
        article class=(if status.local_ai.load_state == "error" { "operational-card local-ai-home-card attention" } else { "operational-card local-ai-home-card" }) data-home-ai-load-state=(&status.local_ai.load_state) data-bind-class="home.ai.state" {
            div class="card-head" aria-label="AI Models" {
                h3 { "AI Models" }
            }
            div class="state-rows state-rows--compact local-ai-home-details" {
                (home_bound_detail_row("Load:", load_label, true, "home.ai.load"))
                (home_bound_detail_row("State:", activity_label, true, "home.ai.activity"))
            }
            div class="state-rows state-rows--compact local-ai-home-models" data-bind-each="home.ai.models" data-bind-replace="true" {
                template {
                    div class="home-detail-row" {
                        span data-bind="state" {}
                        strong data-bind="name" {}
                    }
                }
                @if models.is_empty() {
                    div class="home-detail-row" { span { "Models:" } strong { "No models installed" } }
                } @else {
                    @for model in &models {
                        div class="home-detail-row" { span { (&model.state) } strong { (&model.name) } }
                    }
                }
            }
            div class="state-rows state-rows--compact local-ai-home-resident-engines" {
                @if resident_engines.is_empty() { div class="home-detail-row" { span { "Resident engines:" } strong { "None detected" } } }
                @else {
                    div class="home-detail-row" { span { "Resident engines:" } strong { (resident_engines.len()) } }
                    @for engine in resident_engines { div class="home-detail-row" { span { (&engine.function_label) } strong { (engine.model_filename.as_deref().unwrap_or("Model details unavailable")) } } }
                }
            }
        }
    }
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
        article class=(if needs_attention { "operational-card updates-home-card attention" } else { "operational-card updates-home-card" }) data-home-harmonia-state=(&status.updates.state) data-bind-class="home.updates.state" {
            div class="card-head" aria-label="Updates" {
                h3 { "Updates" }
                strong data-bind="home.updates.readinessRatio" { (ratio) }
            }
            div class="state-rows state-rows--compact updates-home-details" {
                (home_bound_detail_row("Last ran:", &status.updates.last_update_run, false, "home.updates.lastRan"))
                (home_bound_detail_row("updates available:", &available_line, true, "home.updates.pendingUpdates"))
            }
            div class="inline-actions inline-actions--compact updates-home-actions" {
                (action_button(ButtonVariant::Primary, "Check", "check-updates", "/api/actions/check-updates"))
            }
        }
    }
}

fn home_bound_detail_row(label: &str, value: &str, inline_count: bool, bind_path: &str) -> Markup {
    home_bound_detail_row_with_class(label, value, inline_count, bind_path, "")
}

fn home_bound_detail_row_with_class(
    label: &str,
    value: &str,
    inline_count: bool,
    bind_path: &str,
    extra_class: &str,
) -> Markup {
    let row_class = if inline_count {
        "state-row home-detail-row home-detail-row--inline"
    } else {
        "state-row home-detail-row"
    };
    let row_class = if extra_class.is_empty() {
        row_class.to_string()
    } else {
        format!("{row_class} {extra_class}")
    };
    html! {
        div class=(row_class) aria-label=(format!("{label} {value}")) {
            span { (label) }
            strong data-bind=(bind_path) { (value) }
        }
    }
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
        "repair_pending" => "Update needed",
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



