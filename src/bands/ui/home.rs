fn home_view(status: &ConsoleStatus) -> Markup {
    html! {
        section id="view-home" class="view" data-view-panel="home" tabindex="-1" {
            div class="home-operational-grid home-operational-grid--dashboard" {
                (home_storage_card(status))
                (home_load_card())
                (home_network_card(status))
                (home_updates_card(status))
                (home_local_ai_card(status))
            }
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
    html! {
        article class="operational-card load-home-card" aria-label="Load dashboard" data-load-card data-load-retry-ms="5000" data-stale="false" {
            div class="card-head" aria-label="Load" { h3 { "Load" } }
            div class="load-chart-wrap" aria-label="Load history" role="img" {
                span class="load-sparkline-placeholder" { "Gathering history…" }
                svg class="load-sparkline" viewBox="0 0 100 40" preserveAspectRatio="none" aria-hidden="true" {
                    polyline data-load-history-line points="" fill="none" stroke="currentColor" stroke-width="1.5" vector-effect="non-scaling-stroke" {}
                }
            }
            span class="load-staleness" data-load-staleness aria-live="polite" {}
            div class="load-average-readouts" aria-label="Load average" {
                (load_readout("1m", "oneMinute", "home.telemetry.load.oneMinute"))
                (load_readout("5m", "fiveMinute", "home.telemetry.load.fiveMinute"))
                (load_readout("15m", "fifteenMinute", "home.telemetry.load.fifteenMinute"))
            }
            div class="memory-usage" aria-label="System RAM used" {
                div class="storage-bar storage-bar--home" role="progressbar" aria-valuemin="0" aria-valuemax="100" data-bind-attr="aria-valuenow:home.telemetry.memory.usedPercent" data-memory-bar {
                    span class="storage-segment storage-segment--other" data-bind-style="width:home.telemetry.memory.usedPercent" data-memory-used-segment {}
                }
                div class="load-memory-readout" aria-label="RAM used and total" {
                    span { "RAM used:" }
                    strong data-memory-used data-bind="home.telemetry.memory.usedBytes" data-bind-format="bytes" { "—" }
                    span aria-hidden="true" { "/" }
                    strong data-memory-total data-bind="home.telemetry.memory.totalBytes" data-bind-format="bytes" { "—" }
                }
            }
            details class="load-telemetry-disclosure" {
                summary { "More telemetry" }
                div class="load-telemetry-scroll" tabindex="0" role="region" aria-label="Load telemetry details" {
                    div class="load-telemetry-grid" aria-label="Telemetry" {
                        (load_chip_bound("CPU temp", "cpu", "home.telemetry.cpu.temperatureCelsius", "temperature", Some("gte:82")))
                        (load_chip_bound("CPU usage", "cpu-usage", "home.telemetry.cpu.usagePercent", "percent", None))
                        (load_chip_bound("I/O", "io", "home.telemetry.io.pressureAvg10", "pressure", Some("gte:10")))
                        (load_chip_bound("GPU", "gpu", "home.telemetry.gpu.utilizationPercent", "percent", None))
                        (load_chip_bound("GPU temp", "gpu-temperature", "home.telemetry.gpu.temperatureCelsius", "temperature", None))
                        (load_chip_bound("Storage temp", "storage-temperature", "home.telemetry.temperature.storage", "temperature", None))
                        (load_chip_bound("Fan RPM", "fan", "home.telemetry.fans.0.rpm", "text", None))
                        (load_chip_bound("Read/s", "read", "home.telemetry.io.disk.readBytesPerSec", "transfer-rate", Some("gt:0")))
                        (load_chip_bound("Write/s", "write", "home.telemetry.io.disk.writeBytesPerSec", "transfer-rate", Some("gt:0")))
                    }
                }
            }
        }
    }
}

fn load_readout(label: &str, key: &str, bind: &str) -> Markup {
    html! {
        div class="load-average-readout" data-load-readout=(key) {
            span { (label) }
            strong data-load-readout-value=(key) data-bind=(bind) data-bind-format="load-average" { "—" }
        }
    }
}

fn load_chip_bound(label: &str, key: &str, bind: &str, format: &str, state_rule: Option<&str>) -> Markup {
    html! {
        div class="load-chip" data-state="idle" data-load-chip=(key) data-bind-state=(format!("{bind}:{}", state_rule.unwrap_or("unclassified"))) {
            em { (label) }
            strong data-load-chip-value=(key) data-bind=(bind) data-bind-format=(format) { "—" }
        }
    }
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
            (home_network_topology(
                status.network.online,
                matches!(status.network.internet_reachable, Some(true)),
                &status.local_ai.detected_ai,
            ))
        }
    }
}

fn home_network_topology(
    console_online: bool,
    internet_online: bool,
    detected_ai: &[crate::DetectedAi],
) -> Markup {
    let console_tone = if console_online { "ok" } else { "warn" };
    let internet_tone = if internet_online { "ok" } else { "warn" };
    let svg_height = (detected_ai.len() as u32 * 36 + 64).max(96);
    let ai_nodes = detected_ai.iter().enumerate().map(|(index, ai)| {
        let label = if ai.label.trim().is_empty() { "AI" } else { ai.label.trim() };
        let detail = if ai.detail.chars().count() > 30 {
            format!("{}…", ai.detail.chars().take(30).collect::<String>())
        } else {
            ai.detail.clone()
        };
        (ai, 82.0_f64, 22.0 + index as f64 * 36.0, label, detail)
    }).collect::<Vec<_>>();
    html! {
        div class="home-network-topology-scroll" tabindex="0" aria-label="Home network topology diagram" {
        svg class="home-network-topology" viewBox=(format!("0 0 420 {svg_height}")) height=(svg_height) role="img" aria-labelledby="home-network-topology-title home-network-topology-desc" {
            title id="home-network-topology-title" { "Home network topology" }
            desc id="home-network-topology-desc" { "Console connections to local AI and the Internet." }
            @if detected_ai.is_empty() {
                circle class="home-network-topology-node home-network-topology-node--idle" cx="82" cy="48" r="7" {}
                text class="home-network-topology-placeholder" x="100" y="48" { "No AI detected" }
            }
            @for (ai, x, y, label, detail) in &ai_nodes {
                line class=(format!("home-network-topology-edge home-network-topology-edge--{}", if ai.alive { "ok" } else { "warn" })) x1="40" y1="20" x2="74" y2=(y) {}
            }
            line class=(format!("home-network-topology-edge home-network-topology-edge--{internet_tone}")) data-bind-class="home.network.internetReachability" x1="24" y1="20" x2="24" y2=(svg_height - 24) {}
            circle class=(format!("home-network-topology-node home-network-topology-node--{console_tone}")) data-bind-class="home.network.consoleReachability" cx="24" cy="20" r="12" {}
            circle class=(format!("home-network-topology-node home-network-topology-node--{internet_tone}")) data-bind-class="home.network.internetReachability" cx="24" cy=(svg_height - 24) r="12" {}
            text class="home-network-topology-label" x="42" y="8" { "Console" }
            text class="home-network-topology-label" x="42" y=(svg_height - 24) { "Internet" }
            @for (ai, x, y, label, detail) in &ai_nodes {
                @let tone = if ai.alive { "ok" } else { "warn" };
                circle class=(format!("home-network-topology-node home-network-topology-node--{tone}")) cx=(x) cy=(y) r="7" {}
                text class="home-network-topology-label home-network-topology-label--ai" x="98" y=(y - 3.0) {
                    title { (label) }
                    (label)
                }
                text class="home-network-topology-detail" x="98" y=(y + 10.0) {
                    title { (ai.detail) }
                    (detail)
                }
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
            h4 class="updates-home-module-heading" { "Enabled modules" }
            div class="updates-home-module-list" aria-label="Enabled modules" data-bind-each="home.updates.modules" data-bind-replace="true" {
                template {
                    div class="updates-home-module-row" {
                        span class="updates-home-module-indicator" aria-hidden="true" data-bind-show="seekingUpdate" { "↑" }
                        span data-bind="label" {}
                    }
                }
                @for module in enabled_home_update_modules(&status.updates.modules) {
                    div class="updates-home-module-row" {
                        span class="updates-home-module-indicator" aria-hidden="true" {
                            @if module.seeking_update { "↑" }
                        }
                        span { (&module.label) }
                    }
                }
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



