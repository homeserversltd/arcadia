fn header(status: &ConsoleStatus) -> Markup {
    let sync_delta = status.library.unsynced_added
        + status.library.unsynced_changed
        + status.library.unsynced_removed;
    let games_total = status.library.total_detected_games;
    let games_total_label = games_total.to_string();
    let (sync_class, sync_tip) = if status.library.last_sync_state == "running" {
        (
            "warn",
            format!("{games_total} games total · syncing now"),
        )
    } else if status.library.last_sync_state == "error" {
        (
            "bad",
            format!("{games_total} games total · last sync failed"),
        )
    } else if !sync_has_history(status) {
        (
            "idle",
            format!(
                "{games_total} games total · no verified sync receipt; current folders contain {games_total} playable ROM files"
            ),
        )
    } else if status.library.last_sync_state == "success" {
        (
            "idle",
            format!("{games_total} games total · last sync counted {games_total} playable ROM files"),
        )
    } else if status.library.sync_needed || sync_delta > 0 {
        (
            "warn",
            format!("{games_total} games total · folder changes are queued for sync"),
        )
    } else if status.library.sync_state == "unknown" {
        (
            "idle",
            format!("{games_total} games total · sync state is unavailable"),
        )
    } else {
        (
            "good",
            format!("{games_total} games total · no queued sync changes"),
        )
    };
    let sync_label = games_total_label;
    let (updates_label, updates_class, updates_tip) = match status.updates.state.as_str() {
        "available" => (
            "Available".to_string(),
            "warn",
            format!(
                "Update available · {}",
                status
                    .updates
                    .available_version
                    .as_deref()
                    .unwrap_or("version unknown")
            ),
        ),
        "current" => (
            "Current".to_string(),
            "good",
            format!("Harmonia current · {}", status.updates.current_version),
        ),
        "repair_pending" => (
            "Repair pending".to_string(),
            "warn",
            format!(
                "Harmonia blocker · {}",
                harmonia_missing_signal_label(&status.updates.first_missing_signal)
            ),
        ),
        "checking" => (
            "Checking".to_string(),
            "warn",
            "Checking for updates".to_string(),
        ),
        "installing" => (
            "Installing".to_string(),
            "warn",
            "Installing update".to_string(),
        ),
        "error" => (
            "Error".to_string(),
            "bad",
            "Update check failed".to_string(),
        ),
        _ => (
            "Unknown".to_string(),
            "idle",
            "Update state unknown".to_string(),
        ),
    };
    let ai_ready = status.network.lan_ai_reachable || status.local_ai.lan_inference_enabled;
    let ai_label = if ai_ready {
        "Ready on :7777"
    } else {
        "Unavailable"
    };
    let ai_tip = if ai_ready {
        "Local AI listener is ready when called on port 7777"
    } else {
        "Local AI listener is not available"
    };
    html! {
        header class="top-header" {
            div class="product-lockup" {
                div class="product-mark" { "H" }
                div { h1 { "HomeConsole" } }
            }
            div class="header-indicators header-indicators--currentness" aria-label="HomeConsole currentness" {
                (currentness_status_chip("network", "Network", &status.network.connection_type, network_class(status.network.active_type.as_str()), &network_tooltip(status), "network", None))
                (currentness_status_chip("sync", "Games", &sync_label, sync_class, &sync_tip, "sync", Some(games_total)))
                (currentness_status_chip("updates", "Updates", &updates_label, updates_class, &updates_tip, "updates", None))
                (currentness_status_chip("uptime", "Uptime", &status.runtime.machine_uptime, "idle", "Machine uptime", "system", None))
                (currentness_status_chip("local-ai", "AI", ai_label, if ai_ready { "good" } else { "idle" }, ai_tip, "local-ai", None))
                (currentness_status_chip("pin", "Lock", if status.gui_pin.pin_required { "PIN required" } else { "Open" }, if status.gui_pin.pin_required { "warn" } else { "idle" }, if status.gui_pin.pin_required { "PIN required for GUI changes" } else { "GUI changes are open without PIN" }, "access-pin", None))
                (theme_cycle_button())
            }
        }
    }
}

fn theme_cycle_button() -> Markup {
    html! {
        button class="status-badge status-badge--theme status-badge--action" type="button" data-theme-cycle="true" data-theme-current="ember-aubergine" data-tooltip="Toggle theme" aria-label="Toggle theme, current theme Ember Aubergine" {
            span class="chip-icon" aria-hidden="true" { (lucide_icon("palette")) }
            span class="chip-copy" { "Theme" }
            strong class="theme-name" { "ember aubergine" }
        }
    }
}

fn network_class(active_type: &str) -> &'static str {
    match active_type {
        "ethernet" | "wifi" => "good",
        "limited" => "warn",
        "offline" => "bad",
        _ => "idle",
    }
}

fn currentness_status_chip(
    kind: &str,
    label: &str,
    value: &str,
    class: &str,
    help: &str,
    target: &str,
    games_total: Option<u64>,
) -> Markup {
    let icon = chip_icon(kind, value);
    html! {
        @if let Some(total) = games_total {
            button class=(format!("status-badge status-badge--{} status-badge--nav status-badge--currentness", class)) type="button" data-nav-target=(target) data-chip-kind=(kind) data-tooltip=(help) data-games-total=(total.to_string()) aria-label=(format!("{}: {}", label, value)) {
                span class="chip-icon" aria-hidden="true" { (lucide_icon(icon)) }
                span class="chip-copy" { (label) }
                strong data-games-total-value { (value) }
            }
        } @else {
            button class=(format!("status-badge status-badge--{} status-badge--nav status-badge--currentness", class)) type="button" data-nav-target=(target) data-chip-kind=(kind) data-tooltip=(help) aria-label=(format!("{}: {}", label, value)) {
                span class="chip-icon" aria-hidden="true" { (lucide_icon(icon)) }
                span class="chip-copy" { (label) }
                strong { (value) }
            }
        }
    }
}

fn chip_icon(kind: &str, value: &str) -> &'static str {
    match kind {
        "network" => "network",
        "games" => "gamepad-2",
        "updates" => "badge-check",
        "uptime" => "clock-3",
        "local-ai" => "bot",
        "pin" if value == "Open" => "unlock-keyhole",
        "pin" => "lock-keyhole",
        _ => "circle-dot",
    }
}

fn lucide_icon(name: &str) -> Markup {
    // Inline SVG path data is from Lucide, an open-source ISC-licensed icon set.
    let body = match name {
        "network" => {
            r#"<rect x="16" y="16" width="6" height="6" rx="1"/><rect x="2" y="16" width="6" height="6" rx="1"/><rect x="9" y="2" width="6" height="6" rx="1"/><path d="M12 8v4m-7 4v-2a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2v2"/>"#
        }
        "gamepad-2" => {
            r#"<line x1="6" y1="11" x2="10" y2="11"/><line x1="8" y1="9" x2="8" y2="13"/><line x1="15" y1="12" x2="15.01" y2="12"/><line x1="18" y1="10" x2="18.01" y2="10"/><path d="M17.32 5H6.68A4.68 4.68 0 0 0 2 9.68v6.64a2.68 2.68 0 0 0 4.66 1.8l1.7-1.9A2 2 0 0 1 9.85 15h4.3a2 2 0 0 1 1.49.66l1.7 1.9A2.68 2.68 0 0 0 22 15.76V9.68A4.68 4.68 0 0 0 17.32 5Z"/>"#
        }
        "badge-check" => {
            r#"<path d="M3.85 8.62a4 4 0 0 1 4.78-4.77 4 4 0 0 1 6.74 0 4 4 0 0 1 4.78 4.78 4 4 0 0 1 0 6.74 4 4 0 0 1-4.77 4.78 4 4 0 0 1-6.75 0 4 4 0 0 1-4.78-4.77 4 4 0 0 1 0-6.76Z"/><path d="m9 12 2 2 4-4"/>"#
        }
        "clock-3" => r#"<circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16.5 12"/>"#,
        "bot" => {
            r#"<path d="M12 8V4H8"/><rect width="16" height="12" x="4" y="8" rx="2"/><path d="M2 14h2"/><path d="M20 14h2"/><path d="M15 13v2"/><path d="M9 13v2"/>"#
        }
        "unlock-keyhole" => {
            r#"<rect width="18" height="11" x="3" y="11" rx="2"/><path d="M7 11V7a5 5 0 0 1 9.9-1"/><circle cx="12" cy="16" r="1"/>"#
        }
        "lock-keyhole" => {
            r#"<rect width="18" height="11" x="3" y="11" rx="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/><circle cx="12" cy="16" r="1"/>"#
        }
        "palette" => {
            r#"<circle cx="13.5" cy="6.5" r=".5"/><circle cx="17.5" cy="10.5" r=".5"/><circle cx="8.5" cy="7.5" r=".5"/><circle cx="6.5" cy="12.5" r=".5"/><path d="M12 22C6.5 22 2 17.97 2 13c0-5 4.03-9 9-9s9 3.58 9 8c0 2.76-2.24 5-5 5h-1.77c-.88 0-1.6.72-1.6 1.6 0 .38.15.74.41 1.01.26.26.41.62.41.99C12.45 21.37 12.17 22 12 22z"/>"#
        }
        _ => r#"<circle cx="12" cy="12" r="10"/>"#,
    };
    PreEscaped(format!(
        r#"<svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" focusable="false" aria-hidden="true">{}</svg>"#,
        body
    ))
}

fn network_tooltip(status: &ConsoleStatus) -> String {
    if !status.network.online {
        return "Offline".to_string();
    }
    let internet = if status.network.internet_reachable == Some(true) {
        "Internet reachable"
    } else {
        "Internet unavailable"
    };
    if status.network.active_type == "wifi" {
        format!(
            "Wi-Fi {} · {} · {}",
            status.network.ssid.as_deref().unwrap_or("Unknown"),
            status
                .network
                .signal_percent
                .map(|v| format!("{}%", v))
                .unwrap_or_else(|| "unknown signal".to_string()),
            internet
        )
    } else {
        format!("Ethernet · {} · {}", status.network.ip_address, internet)
    }
}

fn title_case_state(state: &str) -> &'static str {
    match state {
        "running" => "Running",
        "stopped" => "Stopped",
        "starting" => "Starting",
        _ => "Unknown",
    }
}

fn sidebar_launcher() -> Markup {
    html! {
        nav class="sidebar-launcher" aria-label="Arcadia pages" {
            @for (view, icon, label) in VIEWS {
                button class="launcher-button" type="button" data-view=(view) aria-controls=(format!("view-{}", view)) {
                    span class="launcher-icon" aria-hidden="true" { (icon) }
                    span class="launcher-label" { (label) }
                }
            }
        }
    }
}

fn view_shell(id: &str, _eyebrow: &str, _title: &str, _explanation: &str, body: Markup) -> Markup {
    html! {
        section id=(format!("view-{}", id)) class="view" data-view-panel=(id) tabindex="-1" {
            (body)
        }
    }
}

