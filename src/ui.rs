use maud::{html, Markup, PreEscaped, DOCTYPE};

use crate::{
    platform_display_name, AiModelStorageStatus, ButtonVariant, ConsoleStatus,
    StorageCategoryStatus, GAMES_ROOT, GAME_SYSTEMS,
};

const VIEWS: [(&str, &str, &str); 8] = [
    ("home", "⌂", "Home"),
    ("sync", "↻", "Sync"),
    ("storage", "▰", "Storage"),
    ("local-ai", "◉", "Local AI"),
    ("network", "◌", "Network"),
    ("access-pin", "●", "Access\nPIN"),
    ("updates", "⬆", "Updates"),
    ("system", "⚙", "System"),
];

pub fn layout(status: &ConsoleStatus) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (status.product) " / Arcadia" }
                script { (theme_boot_script()) }
                link rel="stylesheet" href="/static/app.css";
            }
            body data-ui-schema=(status.ui_contract.schema) data-gui-pin-required=(status.gui_pin.pin_required) {
                (gui_pin_gate(status))
                div id="app" class="app-shell" aria-hidden=(status.gui_pin.pin_required) {
                    (header(status))
                    div class="workspace" {
                        (sidebar_launcher())
                        main class="viewport" aria-live="polite" {
                            (home_view(status))
                            (sync_view(status))
                            (storage_view(status))
                            (ai_model_view(status))
                            (network_view(status))
                            (access_pin_view(status))
                            (updates_view(status))
                            (system_view(status))
                        }
                    }
                }
                (modal_root())
                script src="/static/app.js" {}
            }
        }
    }
}

fn header(status: &ConsoleStatus) -> Markup {
    let sync_delta = status.library.unsynced_added
        + status.library.unsynced_changed
        + status.library.unsynced_removed;
    let (sync_label, sync_class, sync_tip) = if status.library.last_sync_state == "running" {
        (
            "Syncing".to_string(),
            "warn",
            "Game folders are syncing now".to_string(),
        )
    } else if status.library.last_sync_state == "error" {
        (
            "Sync failed".to_string(),
            "bad",
            "Last game sync failed".to_string(),
        )
    } else if !sync_has_history(status) {
        (
            format!("{} ROMs", status.library.total_detected_games),
            "idle",
            format!(
                "No verified sync receipt; current folders contain {} playable ROM files",
                status.library.total_detected_games
            ),
        )
    } else if status.library.last_sync_state == "success" {
        (
            format!("{} ROMs", status.library.total_detected_games),
            "idle",
            format!(
                "Last sync counted {} playable ROM files",
                status.library.total_detected_games
            ),
        )
    } else if status.library.sync_needed || sync_delta > 0 {
        (
            "Sync needed".to_string(),
            "warn",
            "Game folder changes are queued for sync".to_string(),
        )
    } else if status.library.sync_state == "unknown" {
        (
            "Unknown".to_string(),
            "idle",
            "Game sync state is unavailable".to_string(),
        )
    } else {
        (
            "Synced".to_string(),
            "good",
            "No queued game sync changes".to_string(),
        )
    };
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
            format!("Software current · {}", status.updates.current_version),
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
                (currentness_status_chip("network", "Network", &status.network.connection_type, network_class(status.network.active_type.as_str()), &network_tooltip(status), "network"))
                (currentness_status_chip("sync", "Games", &sync_label, sync_class, &sync_tip, "sync"))
                (currentness_status_chip("updates", "Updates", &updates_label, updates_class, &updates_tip, "updates"))
                (currentness_status_chip("uptime", "Uptime", &status.runtime.machine_uptime, "idle", "Machine uptime", "system"))
                (currentness_status_chip("local-ai", "AI", ai_label, if ai_ready { "good" } else { "idle" }, ai_tip, "local-ai"))
                (currentness_status_chip("pin", "Lock", if status.gui_pin.pin_required { "PIN required" } else { "Open" }, if status.gui_pin.pin_required { "warn" } else { "idle" }, if status.gui_pin.pin_required { "PIN required for GUI changes" } else { "GUI changes are open without PIN" }, "access-pin"))
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
) -> Markup {
    let icon = chip_icon(kind, value);
    html! {
        button class=(format!("status-badge status-badge--{} status-badge--nav status-badge--currentness", class)) type="button" data-nav-target=(target) data-chip-kind=(kind) data-tooltip=(help) aria-label=(format!("{}: {}", label, value)) {
            span class="chip-icon" aria-hidden="true" { (lucide_icon(icon)) }
            span class="chip-copy" { (label) }
            strong { (value) }
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

fn home_view(status: &ConsoleStatus) -> Markup {
    html! {
        section id="view-home" class="view" data-view-panel="home" tabindex="-1" {
            (priority_strip(status))
            div class="home-operational-grid" {
                (home_storage_card(status))
                (home_network_card(status))
                (home_local_ai_card(status))
                (home_sync_card(status))
                @if status.arcadia.service != "running" {
                    (home_gamescope_card(status))
                }
            }
            @if status.library.last_sync_state == "error" || status.local_ai.load_state == "error" {
                div class="active-warning-strip" {
                    @if status.library.last_sync_state == "error" {
                        strong { "Sync failed" }
                        span { "Open Sync for the last receipt." }
                        (nav_button("View Sync", "sync"))
                    } @else {
                        strong { "Local AI error" }
                        span { "Open Local AI or unload the failed model." }
                        (nav_button("Open Local AI", "local-ai"))
                    }
                }
            }
        }
    }
}

fn priority_strip(status: &ConsoleStatus) -> Markup {
    let priority: Option<(String, String, Option<&str>, Option<&str>, Option<&str>)> =
        if !status.network.online {
            Some((
                "Network offline".to_string(),
                "Console network is unavailable.".to_string(),
                Some("Manage Network"),
                Some("network"),
                None,
            ))
        } else if status.storage.percent_used >= 90 {
            Some((
                format!("Storage low: {} free", status.storage.free),
                format!("{} used", status.storage.percent),
                Some("Open Storage"),
                Some("storage"),
                None,
            ))
        } else if status.library.last_sync_state == "error" {
            Some((
                "Sync failed".to_string(),
                "Open Sync for the latest receipt.".to_string(),
                Some("View Sync"),
                Some("sync"),
                None,
            ))
        } else if !sync_has_history(status) {
            Some((
                "Sync pending scan".to_string(),
                "No ROM scan has run yet.".to_string(),
                Some("Scan ROMs"),
                None,
                Some("/api/actions/sync-games"),
            ))
        } else if status.library.sync_needed {
            let changes = status.library.unsynced_added
                + status.library.unsynced_changed
                + status.library.unsynced_removed;
            Some((
                format!("{} changes waiting for sync", changes),
                "ROM folder changes are ready to scan.".to_string(),
                Some("Start Sync"),
                None,
                Some("/api/actions/sync-games"),
            ))
        } else if status.updates.state == "available" {
            Some((
                "Update available".to_string(),
                status
                    .updates
                    .available_version
                    .clone()
                    .unwrap_or_else(|| "Review update".to_string()),
                Some("Review Update"),
                Some("updates"),
                None,
            ))
        } else if status.local_ai.load_state == "error" {
            Some((
                "Local AI error".to_string(),
                status
                    .local_ai
                    .selected_model_name
                    .clone()
                    .unwrap_or_else(|| "Model load failed".to_string()),
                Some("Open Local AI"),
                Some("local-ai"),
                None,
            ))
        } else {
            None
        };

    if let Some((state, detail, action, target, endpoint)) = priority {
        html! {
            article class="priority-strip" aria-label="Highest priority console state" {
                strong { (state) }
                span { (detail) }
                @if let Some(label) = action {
                    @if let Some(endpoint) = endpoint {
                        (action_button(ButtonVariant::Primary, label, "sync-games", endpoint))
                    } @else if let Some(target) = target {
                        (nav_button(label, target))
                    }
                }
            }
        }
    } else {
        html! {}
    }
}

fn home_storage_card(status: &ConsoleStatus) -> Markup {
    html! {
        article class=(if status.storage.percent_used >= 90 { "operational-card storage-home-card attention" } else { "operational-card storage-home-card" }) {
            div class="card-head" { h3 { "Storage" } strong { (status.storage.free) " free" } }
            div class="storage-bar storage-bar--home" aria-label="Storage usage by category" {
                span class="storage-segment storage-segment--games" style=(format!("width: {}%", status.storage.games.percent_of_total.max(if status.storage.games.bytes > 0 { 1 } else { 0 }))) title=(format!("Games {}", status.storage.games.size)) {}
                span class="storage-segment storage-segment--artwork" style=(format!("width: {}%", status.storage.artwork.percent_of_total.max(if status.storage.artwork.bytes > 0 { 1 } else { 0 }))) title=(format!("Artwork {}", status.storage.artwork.size)) {}
                span class="storage-segment storage-segment--ai" style=(format!("width: {}%", status.storage.ai_models.percent_of_total.max(if status.storage.ai_models.bytes > 0 { 1 } else { 0 }))) title=(format!("AI Models {}", status.storage.ai_models.size)) {}
                span class="storage-segment storage-segment--other" style=(format!("width: {}%", status.storage.other.percent_of_total.max(if status.storage.other.bytes > 0 { 1 } else { 0 }))) title=(format!("Other {}", status.storage.other.size)) {}
                span class="storage-segment storage-segment--free" style=(format!("width: {}%", 100u8.saturating_sub(status.storage.percent_used))) title=(format!("Free {}", status.storage.free)) {}
            }
            p class="card-line" { (status.storage.percent) " used" }
            div class="storage-mini-rows" {
                (storage_mini_row("Games", &status.storage.games.size, status.storage.games.percent_of_total, status.storage.games.bytes))
                (storage_mini_row("Artwork", &status.storage.artwork.size, status.storage.artwork.percent_of_total, status.storage.artwork.bytes))
                (storage_mini_row("AI Models", &status.storage.ai_models.size, status.storage.ai_models.percent_of_total, status.storage.ai_models.bytes))
                (storage_mini_row("Other", &status.storage.other.size, status.storage.other.percent_of_total, status.storage.other.bytes))
            }
            div class="inline-actions inline-actions--compact" {
                (nav_button("Manage Storage", "storage"))
                (nav_button("Browse Folders", "storage"))
                @if status.storage.artwork.bytes >= 1_000_000_000 { (action_button(ButtonVariant::Secondary, "Clean Artwork", "clear-artwork-cache", "/api/actions/clear-artwork-cache")) }
                @if status.storage.ai_models.bytes > 0 { (nav_button("Manage Models", "local-ai")) }
            }
        }
    }
}

fn home_network_card(status: &ConsoleStatus) -> Markup {
    html! {
        article class=(if status.network.online { "operational-card network-home-card" } else { "operational-card network-home-card attention" }) {
            @if status.network.online {
                div class="card-head" { h3 { "Network" } strong { (status.network.connection_type) } }
                @if status.network.active_type == "wifi" {
                    p class="card-line" { (status.network.ssid.as_deref().unwrap_or("Wi-Fi")) " · " (status.network.signal_percent.map(|v| format!("{}%", v)).unwrap_or_else(|| "Unknown signal".to_string())) }
                    p class="card-line" { (status.network.ip_address) }
                } @else {
                    p class="card-line" { "Ethernet · " (status.network.ethernet_speed_mbps.map(|v| format!("{} Mbps", v)).unwrap_or_else(|| "Unknown speed".to_string())) }
                    p class="card-line" { (status.network.ip_address) }
                }
                div class="reachability-row reachability-row--topology" {
                    (reachability("Link", status.network.online))
                    @if let Some(internet) = status.network.internet_reachable { (reachability("Internet", internet)) }
                }
                div class="inline-actions inline-actions--compact" {
                    (nav_button("Manage Network", "network"))
                    (copy_button("Console URL", status.identity.web_origin.as_str()))
                }
            } @else {
                h3 { "Network offline" }
                p class="card-line" { "Console network is unavailable." }
                (nav_button("Manage Network", "network"))
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
        "Scan ready"
    } else if !sync_has_history(status) {
        "Not scanned"
    } else {
        "Idle"
    };
    let rom_count = rom_count_label(status);
    html! {
        article class="operational-card sync-home-card" {
            div class="card-head" { h3 { "Sync" } strong { (format!("{} · {} ROMs", state, status.library.total_detected_games)) } }
            p class="card-line" { (rom_count) }
            div class="state-rows state-rows--compact" {
                (state_row("Available ROMs", &status.library.total_detected_games.to_string()))
                (state_row("Last scan", &status.library.last_sync))
                @if pending_changes > 0 { (state_row("Folder changes", &pending_changes.to_string())) }
                (state_row("Artwork", &status.library.artwork_status))
            }
            div class="inline-actions inline-actions--compact" {
                (nav_button("Open Sync", "sync"))
                (action_button(ButtonVariant::Secondary, "Scan ROM folders", "sync-games", "/api/actions/sync-games"))
            }
        }
    }
}

fn rom_count_label(status: &ConsoleStatus) -> String {
    format!("{} available ROMs", status.library.total_detected_games)
}

fn home_local_ai_card(status: &ConsoleStatus) -> Markup {
    let state = title_case_state_like(&status.local_ai.load_state);
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
            div class="card-head" { h3 { "Local AI" } strong { (state) } }
            p class="card-line" { (model_line) }
            @if !status.local_ai.available_models.is_empty() {
                label class="compact-select-label" { span { "Model" } select class="compact-select" name="home-local-ai-model" {
                    @for model in &status.local_ai.available_models { option value=(model.id) selected[status.local_ai.selected_model_id.as_deref() == Some(model.id.as_str())] { (model.name) } }
                } }
            }
            div class="state-rows state-rows--compact" {
                @if let Some(accelerator) = accelerator { (state_row("Accelerator", accelerator)) }
                (state_row("LAN", &lan))
                @if status.local_ai.available_models.is_empty() { (state_row("Models", "Not configured")) }
            }
            @if let (Some(used), Some(total)) = (status.local_ai.gpu_memory_used_bytes, status.local_ai.gpu_memory_total_bytes) {
                div class="gpu-bar" aria-label="Accelerator memory usage" { span style=(format!("width: {}%", ((used.saturating_mul(100) / total.max(1)).min(100)))) {} }
            }
            div class="inline-actions inline-actions--compact" {
                @if status.local_ai.load_state == "hot" || status.local_ai.load_state == "loading" { (modal_button(ButtonVariant::Secondary, "Unload", "Unload local AI", "Unload the active local AI model when the backend control is connected.")) }
                @else if !status.local_ai.available_models.is_empty() { (modal_button(ButtonVariant::Primary, "Load", "Load local AI", "Load the selected local AI model when the backend control is connected.")) }
                (nav_button("Open", "local-ai"))
            }
        }
    }
}

fn home_gamescope_card(status: &ConsoleStatus) -> Markup {
    html! {
        article class="operational-card gamescope-home-card attention" {
            div class="card-head" { h3 { "GameScope" } strong { (title_case_state(status.arcadia.service)) } }
            p class="card-line" { "No active game reported." }
            div class="inline-actions inline-actions--compact" {
                (action_button(ButtonVariant::Secondary, "Restart Session", "restart-gamescope", "/api/actions/restart-gamescope"))
            }
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
        _ => "Unknown",
    }
}

fn storage_mini_row(label: &str, value: &str, percent: u8, bytes: u64) -> Markup {
    html! { div class="storage-mini-row" { span { (label) } strong { (value) } em { (percent_label(percent, bytes)) } } }
}

fn percent_label(percent: u8, bytes: u64) -> String {
    if bytes > 0 && percent == 0 {
        "<1%".to_string()
    } else {
        format!("{}%", percent)
    }
}

fn state_row(label: &str, value: &str) -> Markup {
    html! { div class="state-row" { span { (label) } strong { (value) } } }
}

fn reachability(label: &str, ok: bool) -> Markup {
    html! { span class=(if ok { "reachability reachability--ok" } else { "reachability" }) { (label) " " (if ok { "✓" } else { "—" }) } }
}

fn sync_view(status: &ConsoleStatus) -> Markup {
    let storage_blocked = status.storage.health == "Full";
    let storage_low = status.storage.percent_used >= 90;
    let windows_root = r"\\console.home.arpa\games";
    let smb_root = "smb://console.home.arpa/games";
    let ip_value = if status.network.ip_address == "—" {
        "IP unavailable"
    } else {
        status.network.ip_address.as_str()
    };
    let pending_changes = status.library.unsynced_added
        + status.library.unsynced_changed
        + status.library.unsynced_removed;
    let state_label = sync_state_label(status);
    view_shell(
        "sync",
        "",
        "",
        "",
        html! {
            section class="sync-rom-panel sync-rom-panel--decision" data-sync-root="true" data-storage-health=(status.storage.health) data-storage-low=(storage_low) data-storage-blocked=(storage_blocked) data-sync-state=(status.library.last_sync_state) aria-label="Sync" {
                div class="sync-rom-copy" {
                    div class="sync-status-line" role="status" aria-live="polite" aria-atomic="true" {
                        span { "Sync" }
                        strong id="sync-state" data-sync-state=(state_label.to_lowercase().replace(' ', "-")) { (state_label) }
                    }
                    p id="sync-progress-text" aria-live="polite" { (sync_ready_message(status, storage_blocked, pending_changes)) }
                }
                div class="sync-rom-action" {
                    @if storage_blocked || status.library.last_sync_state == "running" {
                        button class="btn btn--primary" type="button" data-button="primary" data-action="sync-games" data-endpoint="/api/actions/sync-games" disabled { (if storage_blocked { "Storage Full" } else { "Sync running" }) }
                    } @else {
                        (action_button(ButtonVariant::Primary, if pending_changes > 0 || !sync_has_history(status) { "Sync now" } else { "Check again" }, "sync-games", "/api/actions/sync-games"))
                    }
                }
            }

            @if storage_blocked {
                div class="warning sync-storage-warning" {
                    strong { "Storage is full. Free space before syncing." }
                    (nav_button("Storage", "storage"))
                }
            } @else if storage_low {
                div class="warning sync-storage-warning" {
                    strong { "Storage is low. Large syncs may fail." }
                    (nav_button("Storage", "storage"))
                }
            }

            section class="sync-folder-source" aria-label="Samba access" {
                div class="sync-folder-source-main" {
                    span { "Add games" }
                    strong { (windows_root) }
                    em { (GAMES_ROOT) }
                }
                div class="inline-actions inline-actions--compact sync-copy-actions" aria-label="Sync copy buttons" {
                    (copy_button("Samba for Windows", windows_root))
                    (copy_button("Samba for Linux", smb_root))
                    (copy_button("IP", ip_value))
                }
                div class="sync-folder-list" aria-label="Queued content folders" {
                    span class="sync-folder-pill sync-folder-pill--queue" { "Queued" " · " code { (pending_changes) } }
                    span class="sync-folder-pill" { "Folder" " · " code { (GAMES_ROOT) } }
                    @for system in GAME_SYSTEMS {
                        span class="sync-folder-pill" { (platform_display_name(system)) " · " code { (format!("games/{}", system)) } }
                    }
                }
            }

            section id="sync-running-panel" class="sync-running-panel" aria-label="Sync progress" hidden[status.library.last_sync_state != "running"] {
                div class="sync-scanner" aria-hidden="true" {
                    span class="sync-scanner-dot" {}
                    span class="sync-scanner-line" {}
                    span class="sync-scanner-file" {}
                }
                div {
                    strong id="sync-running-title" { "Syncing" }
                    p id="sync-running-copy" { "Reading the games folders and updating GameScope entries." }
                }
            }

            section class="sync-result-card" aria-labelledby="sync-result-title" data-sync-result=(sync_result_kind(status)) {
                div class="sync-result-head" {
                    h3 id="sync-result-title" { (sync_result_title(status, pending_changes)) }
                    p id="sync-result-copy" { (sync_result_copy(status, pending_changes)) }
                }
                div class="sync-result-list" {
                    (sync_detail("Queued", &pending_changes.to_string()))
                    (sync_detail("Available ROMs", &status.library.total_detected_games.to_string()))
                    (sync_detail("GameScope", &status.library.total_synced_entries.to_string()))
                    @if status.library.unsynced_added > 0 { (sync_detail("New", &status.library.unsynced_added.to_string())) }
                    @if status.library.unsynced_changed > 0 { (sync_detail("Changed", &status.library.unsynced_changed.to_string())) }
                    @if status.library.unsynced_removed > 0 { (sync_detail("Removed", &status.library.unsynced_removed.to_string())) }
                    @if status.library.artwork_complete > 0 || status.library.artwork_missing > 0 { (sync_detail("Artwork", &status.library.artwork_status)) }
                }
            }

            div id="sync-output-panel" class="collapsible-log sync-output-store" hidden { pre { code id="sync-output" {} } }
            div id="console-action-message" class="message" hidden {}
        },
    )
}

fn sync_has_history(status: &ConsoleStatus) -> bool {
    matches!(
        status.library.last_sync_state.as_str(),
        "success" | "error" | "running"
    ) || status.library.first_sync_completed
}

fn sync_state_label(status: &ConsoleStatus) -> &'static str {
    let pending_changes = status.library.unsynced_added
        + status.library.unsynced_changed
        + status.library.unsynced_removed;
    match status.library.last_sync_state.as_str() {
        "running" => "Syncing",
        "error" => "Sync failed",
        _ if pending_changes > 0 || status.library.sync_needed => "Sync needed",
        _ if !sync_has_history(status) => "Needs first sync",
        _ if status.library.total_detected_games > status.library.total_synced_entries => {
            "Sync recommended"
        }
        _ => "Synced",
    }
}

fn sync_ready_message(
    status: &ConsoleStatus,
    storage_blocked: bool,
    pending_changes: u64,
) -> String {
    if storage_blocked {
        "Storage is full. Free space before syncing.".to_string()
    } else if pending_changes > 0 {
        format!(
            "Sync needed: {} queued change(s) in {}.",
            pending_changes, GAMES_ROOT
        )
    } else if !sync_has_history(status) {
        format!(
            "Needs first sync: add ROMs under {}, then sync.",
            GAMES_ROOT
        )
    } else if status.library.total_detected_games > status.library.total_synced_entries {
        format!(
            "Sync recommended: {} detected, {} in GameScope.",
            status.library.total_detected_games, status.library.total_synced_entries
        )
    } else {
        format!("Synced: 0 queued changes in {}.", GAMES_ROOT)
    }
}

fn sync_result_title(status: &ConsoleStatus, pending_changes: u64) -> &'static str {
    match status.library.last_sync_state.as_str() {
        "running" => "Sync in progress",
        "error" => "Sync failed",
        _ if pending_changes > 0 => "Queued for sync",
        _ if !sync_has_history(status) => "First sync waiting",
        "success" => "Last sync complete",
        _ => "Sync state",
    }
}

fn sync_result_kind(status: &ConsoleStatus) -> &'static str {
    match status.library.last_sync_state.as_str() {
        "success" => "success",
        "error" => "error",
        "running" => "running",
        _ => "idle",
    }
}

fn sync_result_copy(status: &ConsoleStatus, pending_changes: u64) -> String {
    match status.library.last_sync_state.as_str() {
        "error" => "Read the hidden sync output panel for the last failure receipt.".to_string(),
        "running" => format!("Sync is reading {} now.", GAMES_ROOT),
        _ if pending_changes > 0 => format!(
            "{} queued change(s): {} new, {} changed, {} removed under {}.",
            pending_changes,
            status.library.unsynced_added,
            status.library.unsynced_changed,
            status.library.unsynced_removed,
            GAMES_ROOT
        ),
        _ if !sync_has_history(status) => {
            format!(
                "No sync receipt yet. Put ROMs in {} and press Sync now.",
                GAMES_ROOT
            )
        }
        _ => format!("0 queued changes under {}.", GAMES_ROOT),
    }
}

fn storage_view(status: &ConsoleStatus) -> Markup {
    let cleanup_total = status
        .storage
        .cleanup
        .artwork_bytes_clearable
        .saturating_add(status.storage.cleanup.temporary_bytes_clearable)
        .saturating_add(status.storage.cleanup.partial_downloads_bytes_clearable)
        .saturating_add(status.storage.cleanup.old_update_bytes_clearable)
        .saturating_add(status.storage.cleanup.logs_bytes_clearable);
    let diagnostics_count = storage_diagnostics_count(status);
    let classified_bytes = status
        .storage
        .games
        .bytes
        .saturating_add(status.storage.artwork.bytes)
        .saturating_add(status.storage.ai_models.bytes)
        .saturating_add(status.storage.categories.updates.bytes)
        .saturating_add(status.storage.categories.logs.bytes)
        .saturating_add(status.storage.categories.temporary.bytes)
        .saturating_add(status.storage.categories.system.bytes);
    let accounted_bytes = classified_bytes.saturating_add(status.storage.other.bytes);
    let mismatch_copy = if diagnostics_count > 0 {
        storage_mismatch_copy(status, classified_bytes, accounted_bytes)
    } else if status.storage.percent_used >= status.storage.thresholds.low_percent {
        format!(
            "Filesystem reports {} used and {} free. Review cleanup before downloads or sync.",
            status.storage.used, status.storage.free
        )
    } else {
        format!(
            "Filesystem reports {} used. Category scan accounts for {} classified plus {} other.",
            status.storage.used,
            human_or_zero(classified_bytes),
            status.storage.other.size
        )
    };
    view_shell(
        "storage",
        "",
        "",
        "",
        html! {
            section class="storage-appliance storage-appliance--one-pane" aria-label="Storage overview" {
                article class=(if diagnostics_count > 0 { "storage-summary storage-dashboard storage-dashboard--warning" } else { "storage-summary storage-dashboard" }) {
                    div class="storage-command-center" {
                        div class="storage-health-block" {
                            span class=(if diagnostics_count > 0 { "status-pill partial" } else { "status-pill up" }) {
                                @if diagnostics_count > 0 { "Mismatch detected" } @else { "Storage " (status.storage.health) }
                            }
                            strong { (status.storage.free) " free" }
                            em { (status.storage.percent) " used · scanned " (human_scan_time(&status.storage.scanned_at)) }
                        }
                        div class="storage-actions" aria-label="Storage actions" {
                            (action_button(ButtonVariant::Primary, "Rescan", "storage-rescan", "/api/storage/rescan-summary"))
                            @if diagnostics_count > 0 {
                                button class="btn btn--secondary" type="button" data-storage-modal="diagnostics" { "Review mismatch" }
                            } @else if cleanup_total > 0 {
                                button class="btn btn--secondary" type="button" data-storage-modal="cleanup-review" { "Review cleanup" }
                            }
                            button class="btn btn--secondary" type="button" data-storage-modal="locations" { "Managed locations" }
                        }
                    }

                    div class="storage-hero-metrics" aria-label="Capacity summary" {
                        (storage_stat("Total", &status.storage.total))
                        (storage_stat("Used", &status.storage.used))
                        (storage_stat("Free", &status.storage.free))
                        (storage_stat("Scan", &human_scan_time(&status.storage.scanned_at)))
                    }

                    div class="storage-visual-panel" {
                        div class="storage-meter-head" {
                            strong { "Capacity" }
                            span { (status.storage.used) " used of " (status.storage.total) }
                        }
                        (storage_usage_bar(status))
                        div class="storage-legend storage-legend--inline" aria-label="Storage legend" {
                            (storage_legend_item("games", "Games", &status.storage.games.size))
                            (storage_legend_item("artwork", "Artwork", &status.storage.artwork.size))
                            (storage_legend_item("ai", "AI Models", &status.storage.ai_models.size))
                            (storage_legend_item("other", "Other", &status.storage.other.size))
                            (storage_legend_item("free", "Free", &status.storage.free))
                        }
                    }
                }

                article class=(if diagnostics_count > 0 { "storage-alert-panel storage-alert-panel--warning" } else { "storage-alert-panel" }) {
                    strong {
                        @if diagnostics_count > 0 { "Storage mismatch detected" }
                        @else if status.storage.percent_used >= status.storage.thresholds.low_percent { "Storage low" }
                        @else { "Storage accounting matches" }
                    }
                    span { (mismatch_copy) }
                    div class="storage-accounting-grid" aria-label="Storage accounting" {
                        (storage_accounting_fact("Filesystem used", &status.storage.used))
                        (storage_accounting_fact("Category scan", &human_or_zero(classified_bytes)))
                        (storage_accounting_fact("Other / unclassified", &status.storage.other.size))
                        (storage_accounting_fact("Scan time", &format!("{} ms", status.storage.diagnostics.scan_duration_ms)))
                    }
                }

                div class="storage-category-list storage-category-list--dashboard" aria-label="Storage consumption breakdown" {
                    (storage_category_row("🎮", "Games", &status.storage.games, "games"))
                    (storage_category_row("🖼", "Artwork", &status.storage.artwork, "artwork"))
                    (storage_ai_category_row(&status.storage.ai_models))
                    (storage_category_row("⬇", "Updates", &status.storage.categories.updates, "updates"))
                    (storage_category_row("≋", "Logs", &status.storage.categories.logs, "logs"))
                    (storage_category_row("⌁", "Temporary Files", &status.storage.categories.temporary, "temporary"))
                    (storage_category_row("▣", "System", &status.storage.categories.system, "system"))
                    (storage_category_row("◇", "Other", &status.storage.other, "other"))
                    (storage_free_row(status))
                }
            }
        },
    )
}

fn storage_diagnostics_count(status: &ConsoleStatus) -> usize {
    status.storage.diagnostics.missing_dirs.len()
        + status.storage.diagnostics.permission_errors.len()
        + status.storage.diagnostics.warnings.len()
        + status.storage.diagnostics.overlap_warnings.len()
        + status.storage.diagnostics.category_scan_errors.len()
}

fn storage_mismatch_copy(
    status: &ConsoleStatus,
    classified_bytes: u64,
    accounted_bytes: u64,
) -> String {
    let first_warning = status
        .storage
        .diagnostics
        .warnings
        .first()
        .or_else(|| status.storage.diagnostics.overlap_warnings.first())
        .or_else(|| status.storage.diagnostics.category_scan_errors.first())
        .or_else(|| status.storage.diagnostics.missing_dirs.first())
        .map(String::as_str)
        .unwrap_or("Managed storage scan reported a mismatch.");
    format!(
        "{} Filesystem used {}. Managed categories classify {}; accounting total is {}. Rescan, then review mismatch if it remains.",
        first_warning,
        status.storage.used,
        human_or_zero(classified_bytes),
        human_or_zero(accounted_bytes)
    )
}

fn storage_accounting_fact(label: &str, value: &str) -> Markup {
    html! { span class="storage-accounting-fact" { em { (label) } strong { (value) } } }
}

fn storage_legend_item(color: &str, label: &str, value: &str) -> Markup {
    html! { span class=(format!("storage-legend-item storage-legend-item--{}", color)) { em {} strong { (label) } small { (value) } } }
}

fn storage_category_row(
    icon: &str,
    label: &str,
    category: &StorageCategoryStatus,
    color: &str,
) -> Markup {
    html! {
        article class="storage-category-row storage-category-row--inline" data-storage-category=(color) {
            span class="storage-category-icon" { (icon) }
            div class="storage-category-main" {
                strong { (label) }
                small { (category.detail) }
            }
            b { (category.size) }
            em { (percent_label(category.percent_of_total, category.bytes)) }
            div class="storage-category-mini" aria-hidden="true" { span class=(format!("storage-segment--{}", color)) style=(format!("width: {}%", category.percent_of_total.max(if category.bytes == 0 { 0 } else { 1 }))) {} }
            span class=(format!("system-status system-status--{}", state_class(&category.state))) { (title_case_state_like(&category.state)) }
        }
    }
}

fn storage_ai_category_row(category: &AiModelStorageStatus) -> Markup {
    html! {
        article class="storage-category-row storage-category-row--inline" data-storage-category="ai" {
            span class="storage-category-icon" { "◉" }
            div class="storage-category-main" {
                strong { "AI Models" }
                small { (category.detail) }
            }
            b { (category.size) }
            em { (percent_label(category.percent_of_total, category.bytes)) }
            div class="storage-category-mini" aria-hidden="true" { span class="storage-segment--ai" style=(format!("width: {}%", category.percent_of_total.max(if category.bytes == 0 { 0 } else { 1 }))) {} }
            span class="system-status system-status--available" { (category.meta) }
        }
    }
}

fn storage_free_row(status: &ConsoleStatus) -> Markup {
    let free_percent = 100u8.saturating_sub(status.storage.percent_used);
    html! {
        article class="storage-category-row storage-category-row--inline" data-storage-category="free" {
            span class="storage-category-icon" { "○" }
            div class="storage-category-main" {
                strong { "Free" }
                small { "Available capacity for games, artwork, updates, and Local AI models." }
            }
            b { (status.storage.free) }
            em { (percent_label(free_percent, status.storage.free_bytes)) }
            div class="storage-category-mini" aria-hidden="true" { span class="storage-segment--free" style=(format!("width: {}%", free_percent)) {} }
            span class="system-status system-status--available" { "Available" }
        }
    }
}

fn state_class(state: &str) -> &'static str {
    match state {
        "ok" => "available",
        "warning" => "starting",
        "unknown" => "unknown",
        _ => "unknown",
    }
}

fn human_scan_time(scanned_at: &str) -> String {
    if scanned_at.contains('T') {
        "just now".to_string()
    } else {
        scanned_at.to_string()
    }
}

fn storage_usage_bar(status: &ConsoleStatus) -> Markup {
    html! { div class="storage-bar" aria-label="Segmented storage usage" {
        (storage_segment("games", status.storage.games.percent_of_total, status.storage.games.bytes, &status.storage.games.size))
        (storage_segment("artwork", status.storage.artwork.percent_of_total, status.storage.artwork.bytes, &status.storage.artwork.size))
        (storage_segment("ai", status.storage.ai_models.percent_of_total, status.storage.ai_models.bytes, &status.storage.ai_models.size))
        (storage_segment("updates", status.storage.categories.updates.percent_of_total, status.storage.categories.updates.bytes, &status.storage.categories.updates.size))
        (storage_segment("logs", status.storage.categories.logs.percent_of_total, status.storage.categories.logs.bytes, &status.storage.categories.logs.size))
        (storage_segment("temporary", status.storage.categories.temporary.percent_of_total, status.storage.categories.temporary.bytes, &status.storage.categories.temporary.size))
        (storage_segment("system", status.storage.categories.system.percent_of_total, status.storage.categories.system.bytes, &status.storage.categories.system.size))
        span class="storage-segment storage-segment--free" style=(format!("width: {}%", 100u8.saturating_sub(status.storage.percent_used))) title=(format!("Free {}", status.storage.free)) {}
    } }
}

fn storage_segment(class: &str, percent: u8, bytes: u64, size: &str) -> Markup {
    let width = percent.max(if bytes > 0 { 1 } else { 0 });
    html! { span class=(format!("storage-segment storage-segment--{}", class)) style=(format!("width: {}%", width)) title=(size) {} }
}

fn human_or_zero(bytes: u64) -> String {
    if bytes == 0 {
        "0 B".to_string()
    } else {
        crate::human_size(bytes)
    }
}

fn ai_model_view(status: &ConsoleStatus) -> Markup {
    let selected_name = status
        .local_ai
        .selected_model_name
        .as_deref()
        .unwrap_or("No model selected");
    let loaded_name = status
        .local_ai
        .loaded_model_name
        .as_deref()
        .unwrap_or("No model loaded");
    let port = status.local_ai.lan_inference_port.unwrap_or(7777);
    let endpoint = format!(
        "{}:{}",
        status.identity.web_origin.trim_end_matches('/'),
        port
    );
    let base_url = format!("{}/v1", endpoint);
    let api_ready = status.local_ai.lan_inference_enabled;
    let model_count = status.local_ai.available_models.len();
    let selected_present = status.local_ai.selected_model_id.is_some();
    let model_loaded = matches!(
        status.local_ai.load_state.as_str(),
        "hot" | "loaded" | "running"
    ) || status.local_ai.loaded_model_id.is_some()
        || status.local_ai.loaded_model_name.is_some();
    let model_state =
        local_ai_model_state_label(status, model_loaded, selected_present, model_count);
    let model_state_class =
        local_ai_state_class(&status.local_ai.load_state, model_loaded, model_count);
    let api_state = if api_ready {
        "API reachable"
    } else {
        "API not listening"
    };
    let access_state = if api_ready {
        "Trusted LAN enabled"
    } else {
        "Console only"
    };
    let next_action = if model_count == 0 {
        "Import a GGUF model"
    } else if !selected_present {
        "Select a model"
    } else if !model_loaded {
        "Load the selected model"
    } else if !api_ready {
        "Test or enable API access"
    } else {
        "Copy the client endpoint"
    };
    view_shell(
        "local-ai",
        "",
        "",
        "",
        html! {
            section class=(format!("local-ai-hero local-ai-hero--{}", model_state_class)) aria-label="Local AI status" data-ai-auto-refresh="true" {
                div class="local-ai-orb" aria-hidden="true" { "◉" }
                div class="local-ai-hero-copy" {
                    span { "Local AI" }
                    strong data-ai-model-state="true" { (model_state) }
                    p { (next_action) " · " (if api_ready { "OpenAI-compatible API is reachable on the trusted home LAN." } else { "No client endpoint is reachable until a model is serving." }) }
                }
                div class="local-ai-hero-endpoint" {
                    span { "Client endpoint" }
                    code id="ai-endpoint-readback" data-ai-endpoint=(if api_ready { endpoint.as_str() } else { "" }) {
                        (if api_ready { endpoint.as_str() } else { "No active endpoint" })
                    }
                    @if api_ready { (copy_button("Copy endpoint", &endpoint)) }
                    @else { button class="btn btn--secondary" type="button" disabled title="Load a model and start the API before copying an endpoint." { "Copy endpoint" } }
                }
            }

            section class="local-ai-section ai-manager-section local-ai-card local-ai-card--model" aria-label="Model control" {
                div class="local-ai-card-head" {
                    strong { "Model control" }
                    span class=(format!("system-status system-status--{}", model_state_class)) { (model_state) }
                }
                div class="local-ai-state-list" {
                    (ai_state_tile("Selected", selected_name, if selected_present { "Ready to load" } else { "Choose or import a model" }, "selected"))
                    (ai_state_tile("Serving now", loaded_name, if model_loaded { "Available for client calls" } else { "No model invoked" }, "loaded"))
                    (ai_state_tile("Model library", &format!("{} installed", model_count), if model_count == 0 { "Empty" } else { "Available" }, "library"))
                    @if let Some(accelerator) = status.local_ai.gpu_memory.as_deref() { (ai_state_tile("Accelerator", accelerator, "Read from backend telemetry", "accelerator")) }
                }
                div class="inline-actions inline-actions--compact" {
                    @if status.local_ai.available_models.is_empty() { (nav_focus_button("Import model", "local-ai", "local-ai-import")) }
                    @else if !selected_present { (nav_focus_button("Choose model", "local-ai", "installed-models")) }
                    @else if !model_loaded { button class="btn btn--primary" type="button" data-ai-action="model-load" data-model-id=(status.local_ai.selected_model_id.as_deref().unwrap_or("")) { "Load model" } }
                    @if model_loaded { button class="btn btn--secondary" type="button" data-ai-action="model-unload" { "Unload" } }
                    button class="btn btn--secondary" type="button" data-ai-logs="true" { "Open logs" }
                }
                @if model_count == 0 {
                    div class="local-ai-empty" { strong { "No GGUF model installed" } p { "Import a model file or fetch a compatible Hugging Face GGUF before enabling client access." } }
                }
            }

            section id="local-ai-inference" class="local-ai-section ai-manager-section local-ai-card local-ai-card--access" aria-label="API access" tabindex="-1" {
                div class="local-ai-card-head" {
                    strong { "API access" }
                    span class=(format!("system-status system-status--{}", if api_ready { "available" } else { "disabled" })) { (api_state) }
                }
                div class="local-ai-access-grid" {
                    (ai_state_tile("Internal API", if api_ready { "Listening" } else { "Off" }, if api_ready { "Health test can run now" } else { "Load a model before client calls" }, "api"))
                    (ai_state_tile("LAN API", access_state, if api_ready { "Trusted LAN only" } else { "Disabled until explicitly enabled" }, "lan"))
                    (ai_state_tile("Port", &port.to_string(), "Saved HomeConsole Local AI port", "port"))
                    (ai_state_tile("OpenAI base URL", if api_ready { &base_url } else { "Unavailable" }, if api_ready { "Use this in clients" } else { "No base URL until API listens" }, "endpoint"))
                }
                div class="inline-actions inline-actions--compact" {
                    button class="btn btn--primary" type="button" data-ai-action="inference-enable" disabled[model_count == 0] title=(if model_count == 0 { "Install a GGUF model before enabling API access." } else { "Enable console-local API mode." }) { "API on" }
                    button class="btn btn--secondary" type="button" data-ai-action="inference-disable" { "API off" }
                    button class="btn btn--secondary" type="button" data-ai-action="inference-test" { "Test API" }
                    @if api_ready { (copy_button("Copy base URL", &base_url)) } @else { button class="btn btn--secondary" type="button" disabled title="No API endpoint is reachable yet." { "Copy base URL" } }
                }
                p class="local-ai-help" { "Internal mode keeps the service on this console. LAN mode exposes only the saved port to trusted home-network clients." }
            }

            section class="local-ai-section ai-manager-section local-ai-card local-ai-card--config" aria-label="Port management" {
                div class="local-ai-card-head" {
                    strong { "Port management" }
                    span class="system-status system-status--unknown" id="ai-port-state" data-active-port=(port) { "Active " (port) }
                }
                form class="settings-form settings-form--inline local-ai-port-form" id="ai-lan-form" data-active-port=(port) {
                    label { span { "LAN/API port" } input class="field" name="port" type="number" inputmode="numeric" min="1024" max="65535" value=(port) aria-describedby="ai-port-help"; }
                    label { span { "LAN CIDR" } input class="field" name="lanCidr" value="192.168.123.0/24" autocomplete="off" aria-describedby="ai-port-help"; }
                    p id="ai-port-help" class="local-ai-help" { "Save validates the port without exposing LAN. Enable LAN applies the saved port to trusted-home-LAN access." }
                    div class="inline-actions inline-actions--compact" {
                        button class="btn btn--primary" type="submit" data-ai-port-save="true" { "Save port" }
                        button class="btn btn--secondary" type="button" data-ai-port-revert="true" { "Revert" }
                        button class="btn btn--secondary" type="button" data-ai-action="lan-enable" disabled[!model_loaded] title=(if model_loaded { "Expose Local AI on the trusted LAN." } else { "Load a model before exposing LAN access." }) { "Enable LAN" }
                        button class="btn btn--secondary" type="button" data-ai-action="lan-disable" { "Disable LAN" }
                    }
                }
            }

            section id="local-ai-import" class="local-ai-section ai-manager-section ai-manager-section--desktop-detail" aria-label="Model import" tabindex="-1" {
                form class="settings-form" id="ai-import-form" enctype="multipart/form-data" {
                    label { span { "Import GGUF model" } input class="field" type="file" name="model" accept=".gguf"; }
                    div class="inline-actions" { button class="btn btn--primary" type="submit" { "Import model" } button class="btn btn--secondary" type="button" data-ai-action="models-rescan" { "Rescan storage" } }
                    progress id="ai-import-progress" max="100" value="0" hidden {}
                }
            }

            section class="local-ai-section ai-manager-section ai-manager-section--desktop-detail" aria-label="Model library" {
                div class="model-grid" {
                    @if status.local_ai.available_models.is_empty() {
                        article class="model-card" { strong class="model-name" { "No models installed" } span class="model-filename" { "Import a local .gguf file or download a compatible Hugging Face file." } div class="inline-actions" { (nav_focus_button("Import model", "local-ai", "local-ai-import")) (nav_focus_button("Get GGUF", "local-ai", "get-models")) } }
                    } @else {
                        @for model in &status.local_ai.available_models {
                            (installed_model_card(model, status))
                        }
                    }
                }
            }

            section id="get-models" class="local-ai-section ai-manager-section ai-manager-section--desktop-detail" aria-label="Get GGUF" tabindex="-1" {
                article class="form-card" data-hf-installer="true" {
                    h3 { "Hugging Face GGUF" }
                    label { span { "Repository" } input class="field" name="repoId" placeholder="TheBloke/example-GGUF" autocomplete="off"; }
                    label { span { "File" } input class="field" name="filename" placeholder="example.Q4_K_M.gguf" autocomplete="off"; }
                    label { span { "Revision" } input class="field" name="revision" placeholder="main" autocomplete="off"; }
                    div class="inline-actions" { button class="btn btn--secondary" type="button" data-ai-action="hf-list-files" { "Fetch files" } button class="btn btn--primary" type="button" data-ai-action="hf-download" { "Download" } }
                    div id="hf-file-results" class="diagnostics-results" {}
                }
            }

            section class="local-ai-section ai-manager-section ai-manager-section--desktop-detail" aria-label="Client handoff" {
                div class="system-field-grid" {
                    (system_field("Hermes/Pi base URL", if api_ready { &base_url } else { "Enable API first" }))
                    (system_field("Token", "Configured/redacted by backend"))
                    (system_field("Secret receipts", "Redacted"))
                }
                div class="inline-actions" { button class="btn btn--secondary" type="button" data-ai-action="token-generate" { "Generate token" } button class="btn btn--danger" type="button" data-ai-action="token-revoke" { "Revoke token" } }
            }

            section class="local-ai-section ai-manager-section ai-manager-section--desktop-detail" aria-label="Settings" {
                form class="settings-form settings-form--inline" id="ai-settings-form" {
                    label { span { "Context" } input class="field" name="contextSize" type="number" value="4096" min="512" max="262144"; }
                    @if status.local_ai.gpu_memory.is_some() { label { span { "Accelerator layers" } input class="field" name="gpuLayers" type="number" value="-1" min="-1" max="999"; } }
                    label { span { "Threads" } input class="field" name="threads" type="number" value="0" min="0" max="256"; }
                    label { span { "Batch" } input class="field" name="batch" type="number" value="512" min="1" max="8192"; }
                    label class="model-choice" { input type="checkbox" name="startApiOnBoot"; span { "Start API on boot" } }
                    label class="model-choice" { input type="checkbox" name="autoLoadLastModel"; span { "Auto-load last model" } }
                    button class="btn btn--primary" type="submit" { "Save settings" }
                }
            }

            section class="local-ai-section ai-manager-section ai-manager-section--desktop-detail" aria-label="Hardware and storage" {
                @if let (Some(used), Some(total)) = (status.local_ai.gpu_memory_used_bytes, status.local_ai.gpu_memory_total_bytes) {
                    (meter_block("Accelerator", &human_bytes(used), &human_bytes(total), used, total))
                } @else { div class="empty-state" { strong { "No accelerator telemetry source reported" } } }
                (meter_block("AI model storage", &status.storage.ai_models.size, &status.storage.free, status.storage.ai_models.bytes, status.storage.total_bytes.max(1)))
                div class="inline-actions" { (nav_button("Open Storage", "storage")) }
            }

            section class="local-ai-section ai-manager-section ai-manager-section--desktop-detail" aria-label="Diagnostics" {
                div id="ai-activity" class="system-field-grid" { (system_field("Operation", if api_ready { "serving client calls" } else { "idle" })) (system_field("Last error", "Read from /api/ai/state")) }
                details class="collapsible-log" { summary { "llama.cpp update" } pre { code { "Read from backend logs." } } }
                details class="collapsible-log" { summary { "Model import/load" } pre { code { "Read from backend logs." } } }
                details class="collapsible-log" { summary { "Nginx/firewall" } pre { code { "Read from backend receipts." } } }
            }
            div id="ai-message" class="message" hidden {}
        },
    )
}

fn local_ai_model_state_label(
    status: &ConsoleStatus,
    model_loaded: bool,
    selected_present: bool,
    model_count: usize,
) -> &'static str {
    if status.local_ai.load_state == "error" {
        "Backend error"
    } else if model_loaded {
        "Model loaded"
    } else if selected_present {
        "Model selected, not loaded"
    } else if model_count > 0 {
        "Models installed, none selected"
    } else {
        "No model loaded"
    }
}

fn local_ai_state_class(load_state: &str, model_loaded: bool, model_count: usize) -> &'static str {
    if load_state == "error" {
        "error"
    } else if model_loaded {
        "available"
    } else if model_count > 0 {
        "partial"
    } else {
        "disabled"
    }
}

fn ai_state_tile(label: &str, value: &str, detail: &str, kind: &str) -> Markup {
    html! {
        div class="local-ai-state-tile" data-ai-tile=(kind) {
            span { (label) }
            strong { (value) }
            em { (detail) }
        }
    }
}

fn installed_model_card(model: &crate::LocalAiModelStatus, status: &ConsoleStatus) -> Markup {
    let selected = status.local_ai.selected_model_id.as_deref() == Some(model.id.as_str());
    let hot = status.local_ai.loaded_model_id.as_deref() == Some(model.id.as_str());
    html! { article id="installed-models" class=(if selected { "model-card model-card--selected" } else { "model-card" }) {
        strong class="model-name" { (model.name) @if model.is_recommended { " · Recommended" } }
        span class="model-filename" { (model.filename) }
        div class="model-meta-row" {
            (model_meta("Size", &model.size))
            (model_meta("Quantization", model.quantization.as_deref().unwrap_or("Unknown")))
            (model_meta("Use", model.recommended_use.unwrap_or("Balanced")))
            (model_meta("State", if hot { "Hot" } else if selected { "Cold" } else { "Installed" }))
        }
        div class="inline-actions inline-actions--compact" {
            @if !selected { button class="btn btn--secondary" type="button" data-ai-action="model-select" data-model-id=(model.id) { "Select" } }
            @if !hot { button class="btn btn--primary" type="button" data-ai-action="model-load" data-model-id=(model.id) { "Load" } }
            @if hot { button class="btn btn--secondary" type="button" data-ai-action="model-unload" { "Unload" } span class="model-status" { "Unload before removing this model." } }
            @else { button class="btn btn--danger" type="button" data-ai-action="model-remove" data-model-id=(model.id) data-filename=(model.filename) { "Remove" } }
        }
    } }
}

fn meter_block(label: &str, used: &str, total: &str, used_bytes: u64, total_bytes: u64) -> Markup {
    html! { div class="storage-summary ai-meter" { div class="card-head" { h3 { (label) } strong { (used) " / " (total) } } div class="storage-bar" { span class="storage-segment storage-segment--ai" style=(format!("width: {}%", ((used_bytes.saturating_mul(100) / total_bytes.max(1)).min(100)))) {} } } }
}

fn human_bytes(bytes: u64) -> String {
    let gib = bytes as f64 / 1024.0 / 1024.0 / 1024.0;
    if gib >= 1.0 {
        format!("{:.1} GB", gib)
    } else {
        format!("{} MB", bytes / 1024 / 1024)
    }
}

fn network_view(status: &ConsoleStatus) -> Markup {
    let console_url = status.identity.web_origin.as_str();
    view_shell(
        "network",
        "",
        "",
        "",
        html! {
            div class="network-dashboard" {
                article class="network-section network-section--summary" data-network-current="true" {
                    div class="network-section-head" {
                        strong class="network-summary-title" { (connection_summary_title(status)) }
                        span class=(format!("system-status system-status--{}", if status.network.online { "available" } else { "disabled" })) { (if status.network.online { "Online" } else { "Offline" }) }
                    }
                    div class="network-compact-grid" {
                        (system_field("Active connection", &status.network.connection_type))
                        (system_field("IP address", &status.network.ip_address))
                        (system_field("Gateway", status.network.gateway.as_deref().unwrap_or("Unknown")))
                        (system_field("LAN", if status.network.console_reachable { "Reachable" } else { "Unavailable" }))
                        (system_field("Internet", internet_label(status.network.internet_reachable)))
                        (system_field("DNS", &status.network.dns_status))
                        (system_field("Hostname", &status.identity.hostname))
                        (system_field("Web console URL", console_url))
                    }
                    div class="inline-actions inline-actions--compact" {
                        (copy_button("Copy URL", console_url))
                        @if status.network.ip_address != "—" { (copy_button("Copy IP", &status.network.ip_address)) }
                    }
                }

                @if status.network.wifi_adapter_available {
                    article id="wifi-management" class="network-section network-card" tabindex="-1" aria-label="Wi-Fi" {
                        div class="network-section-head" {
                            strong { "Wi-Fi" }
                            span class="system-status system-status--available" { (wifi_status_label(status)) }
                        }
                        div class="network-summary-lines" {
                            @if status.network.active_type == "wifi" {
                                span { (status.network.ssid.as_deref().unwrap_or("Wi-Fi")) " · " (status.network.signal_percent.map(|v| format!("{}% signal", v)).unwrap_or_else(|| "Signal unavailable".to_string())) }
                            } @else {
                                span { "Available for wireless setup." }
                            }
                        }
                        div class="inline-actions inline-actions--compact" {
                            button class="btn btn--primary" type="button" data-network-action="choose-wifi" { "Choose Network" }
                            button class="btn btn--secondary" type="button" data-network-action="scan-wifi" { "Scan" }
                            button class="btn btn--secondary" type="button" data-open-hidden-wifi="true" { "Join Hidden Network" }
                            button class="btn btn--secondary" type="button" data-network-action="wifi-toggle" data-enabled=(if status.network.active_type == "wifi" { "false" } else { "true" }) { (if status.network.active_type == "wifi" { "Turn Off" } else { "Turn On" }) }
                        }
                        div id="wifi-message" class="message" hidden {}
                    }
                }

                article class="network-section network-card" aria-label="Wired LAN" {
                    div class="network-section-head" {
                        strong { "Wired LAN" }
                        span class=(format!("system-status system-status--{}", if status.network.ethernet_connected { "available" } else { "disabled" })) {
                            (if status.network.ethernet_connected { "Connected" } else { "Cable disconnected" })
                        }
                    }
                    @if status.network.ethernet_available {
                        div class="network-compact-grid network-compact-grid--small" {
                            (system_field("Link", if status.network.ethernet_connected { "Connected" } else { "Cable disconnected" }))
                            (system_field("Speed", &status.network.ethernet_speed_mbps.map(|v| format!("{} Mbps", v)).unwrap_or_else(|| "Unknown".to_string())))
                            (system_field("Mode", if status.network.ethernet_dhcp { "DHCP" } else { "Manual" }))
                            (system_field("IP address", &status.network.ip_address))
                            (system_field("Gateway", status.network.gateway.as_deref().unwrap_or("Unknown")))
                        }
                        div class="inline-actions inline-actions--compact" {
                            button class="btn btn--secondary" type="button" data-open-wired-details="true" { "Details" }
                            button class="btn btn--secondary" type="button" data-network-action="renew-dhcp" { "Renew DHCP Lease" }
                            button class="btn btn--secondary" type="button" data-open-ip-settings="true" { "IP Settings" }
                        }
                    } @else {
                        div class="empty-state" { strong { "Cable disconnected" } p { "Connect Ethernet or choose Wi-Fi." } }
                    }
                }

                article class="network-section network-card network-card--wide" aria-label="Services" {
                    div class="network-section-head" { strong { "Services" } }
                    div class="network-service-list" {
                        (network_service_row("Web Console", "Available", console_url, html! { (copy_button("Copy URL", console_url)) }))
                        (network_service_row("LAN AI", if status.network.lan_ai_reachable { "Available" } else { "Disabled" }, if status.network.lan_ai_reachable { ":7777" } else { "" }, html! { (nav_focus_button("Open Local AI", "local-ai", "local-ai-inference")) }))
                        (network_service_row("SSH", "Disabled", "", html! { (nav_button("Open System", "system")) }))
                    }
                }

            details class="network-section diagnostics-panel sync-desktop-detail" {
                summary { "Diagnostics" }
                div class="inline-actions inline-actions--compact" data-diagnostics-actions="true" {
                    button class="btn btn--secondary" type="button" data-diagnostic="gateway" { "Test Gateway" }
                    button class="btn btn--secondary" type="button" data-diagnostic="dns" { "Test DNS" }
                    button class="btn btn--secondary" type="button" data-diagnostic="internet" { "Test Internet" }
                    button class="btn btn--secondary" type="button" data-diagnostic="lan-ai" { "Test LAN AI" }
                }
                div id="diagnostics-results" class="diagnostics-results" {}
            }
        }
        },
    )
}

fn wifi_status_label(status: &ConsoleStatus) -> &'static str {
    if !status.network.wifi_adapter_available {
        "Unavailable"
    } else if status.network.active_type == "wifi" {
        "On"
    } else {
        "Available"
    }
}

fn connection_summary_title(status: &ConsoleStatus) -> &'static str {
    match status.network.active_type.as_str() {
        "ethernet" => "Connected by Ethernet",
        "wifi" => "Connected to Wi-Fi",
        "limited" => "Connected to LAN",
        "offline" => "Offline",
        _ => "Unknown",
    }
}

fn internet_label(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "Reachable",
        Some(false) => "Unavailable",
        None => "Unknown",
    }
}

fn network_service_row(name: &str, state: &str, route: &str, action: Markup) -> Markup {
    html! { div class="network-service-row" { span { strong { (name) } em { (route) } } b class=(format!("system-status system-status--{}", state.to_lowercase())) { (state) } span class="system-row-actions" { (action) } } }
}

fn settings_action_row(
    title: &str,
    description: &str,
    icon: Option<&str>,
    status: Option<(&str, &str)>,
    action: Markup,
    destructive: bool,
) -> Markup {
    html! {
        article class=(if destructive { "settings-action-row settings-action-row--destructive" } else { "settings-action-row" }) {
            @if let Some(icon_text) = icon {
                span class="settings-action-row__icon" aria-hidden="true" { (icon_text) }
            }
            div class="settings-action-row__copy" {
                div class="settings-action-row__titleline" {
                    h3 { (title) }
                    @if let Some((label, class_name)) = status {
                        span class=(format!("system-status {class_name}")) { (label) }
                    }
                }
                p { (description) }
            }
            div class="settings-action-row__action" { (action) }
        }
    }
}

fn access_pin_view(status: &ConsoleStatus) -> Markup {
    view_shell(
        "access-pin",
        "Router-style access",
        "Access & PIN",
        "Router-style PIN management for the HomeConsole access gate.",
        html! {
            div class="access-pin-panel" {
                article class="access-pin-card access-pin-card--mode" data-module="gui-pin-access" {
                    div class="access-pin-state" {
                        span class="access-pin-icon" aria-hidden="true" { "●" }
                        div {
                            h3 { "Access mode" }
                            strong data-pin-mode-label="true" { (if status.gui_pin.pin_required { "PIN required" } else { "Open without PIN" }) }
                            p data-pin-mode-copy="true" { (if status.gui_pin.pin_required { "PIN required before accessing HomeConsole." } else { "HomeConsole opens without a PIN." }) }
                        }
                    }
                    label class="pin-toggle" {
                        input type="checkbox" role="switch" name="pin_required" data-pin-required-toggle="true" checked[status.gui_pin.pin_required];
                        span class="pin-toggle-track" aria-hidden="true" { span class="pin-toggle-thumb" {} }
                        span class="pin-toggle-label" { "Require PIN for console access" }
                    }
                }
                article class="access-pin-card access-pin-card--change" {
                    h3 { "Change access PIN" }
                    p { "Enter the current PIN and choose the new access PIN. Saved PIN values are never shown." }
                    form id="gui-pin-change-form" class="settings-form settings-form--pin" autocomplete="off" {
                        label { span { "Current PIN" } input class="field" type="password" name="current_pin" autocomplete="current-password" required; }
                        div class="pin-form-row" {
                            label { span { "New PIN" } input class="field" type="password" name="new_pin" autocomplete="new-password" required minlength="4"; }
                            label { span { "Confirm new PIN" } input class="field" type="password" name="confirm_pin" autocomplete="new-password" required minlength="4"; }
                        }
                        div id="gui-pin-change-message" class="message" hidden {}
                        button class="btn btn--primary" type="submit" { "Change access PIN" }
                    }
                }
                (settings_action_row(
                    "Default / reset PIN",
                    "Reset restores only the active access PIN to the configured factory/default value. Games, settings, storage, and the operating system stay unchanged.",
                    Some("!"),
                    Some(if status.gui_pin.default_reset_available { ("Default reset available", "system-status--ok") } else { ("Reset helper missing", "system-status--disabled") }),
                    html! {
                        button class="btn btn--danger" type="button" data-gui-pin-reset-default="true" disabled[!status.gui_pin.default_reset_available] { "Reset PIN to default" }
                        div id="gui-pin-reset-message" class="message" hidden {}
                    },
                    true,
                ))
            }
        },
    )
}

fn updates_view(status: &ConsoleStatus) -> Markup {
    view_shell(
        "updates",
        "",
        "",
        "",
        html! {
            div class="harmonia-panel" data-harmonia-updates="true" {
                article class=(format!("harmonia-currentness harmonia-currentness--{}", status.updates.state)) {
                    div class="harmonia-orb" aria-hidden="true" { "H" }
                    div class="harmonia-currentness-copy" {
                        strong { (harmonia_state_label(&status.updates.state)) }
                        span { (status.updates.profile_id) " / " (status.updates.identity) " · " (status.updates.first_missing_signal) }
                    }
                    div class="harmonia-currentness-actions" {
                        (action_button(ButtonVariant::Secondary, "Check state", "check-updates", "/api/actions/check-updates"))
                        (action_button(ButtonVariant::Primary, "Make harmonious", "update-gui", "/api/actions/update-gui"))
                        button class="btn btn--secondary" type="button" data-harmonia-module-menu="true" { "Modules" }
                        button class="btn btn--secondary" type="button" data-harmonia-ledger-open="true" { "Ledger" }
                    }
                }
                div class="harmonia-metrics" {
                    (status_card("Arcadia", status.arcadia.version, "GUI runtime"))
                    (status_card("Modules", &format!("{} enabled", status.updates.modules.iter().filter(|module| module.enabled).count()), "Profile spine"))
                    (status_card("Operations", &status.updates.operation_count.to_string(), "Last Harmonia run"))
                    (status_card("Receipt", &receipt_short_name(&status.updates.latest_receipt), "Suite receipt"))
                }
                div class="harmonia-module-grid" data-harmonia-module-grid="true" {
                    @for module in &status.updates.modules {
                        (harmonia_module_row(module))
                    }
                }
                details class="collapsible-log harmonia-receipts" {
                    summary { "Receipts" }
                    div class="system-field-grid" {
                        (system_field("Suite", &status.updates.latest_receipt))
                        (system_field("Check", &status.updates.latest_check_receipt))
                        (system_field("Module root", &status.updates.module_root))
                    }
                }
                div id="console-action-message" class="message" hidden {}
            }
        },
    )
}

fn harmonia_state_label(state: &str) -> &'static str {
    match state {
        "current" => "Harmonia current",
        "repair_pending" => "Repair pending",
        "checking" => "Checking",
        "installing" => "Making harmonious",
        "available" => "Update available",
        "error" => "Harmonia error",
        _ => "Harmonia unknown",
    }
}

fn receipt_short_name(path: &str) -> String {
    path.rsplit('/')
        .take(2)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("/")
}

fn harmonia_module_row(module: &crate::HarmoniaModuleStatus) -> Markup {
    html! {
        article class=(format!("harmonia-module harmonia-module--{}", module.state)) data-harmonia-module=(module.id) data-module-enabled=(module.enabled) {
            div {
                strong { (module.label) }
                span { (module.id) }
            }
            b class=(format!("system-status system-status--{}", if module.enabled && module.present { "available" } else if module.enabled { "error" } else { "disabled" })) {
                (if module.enabled { if module.present { "Enabled" } else { "Missing" } } else { "Off" })
            }
            button class="btn btn--secondary" type="button" data-harmonia-module-toggle=(module.id) data-enabled=(module.enabled) {
                (if module.enabled { "Turn off" } else { "Turn on" })
            }
        }
    }
}

fn system_view(status: &ConsoleStatus) -> Markup {
    let ssh = &status.system.ssh;
    let trust = &status.system.trust;
    view_shell(
        "system",
        "Machine status",
        "System",
        "",
        html! {
            (system_power_panel())
            section class="system-grid system-grid--admin" aria-label="System administration" {
                article class="system-card system-card--ssh" {
                    div class="system-card-band" { strong { "Remote Access" } b class=(status_class(&ssh.service_state)) { (title_case_state(&ssh.service_state)) } }
                    div class="system-field-grid" {
                        (system_field("SSH service", &title_case_state(&ssh.service_state)))
                        (system_field("Password login", &title_case_state(&ssh.password_auth)))
                        (system_field("Hostname", &ssh.hostname))
                        (system_field("LAN IP address", &ssh.lan_ip))
                        (system_field("Username", &ssh.username))
                        (system_field("Authorized keys", &ssh.authorized_keys_path))
                    }
                    (command_box("Example command", &ssh.command))
                    div class="inline-actions" {
                        (action_button(ButtonVariant::Secondary, "Enable SSH", "enable-ssh", "/api/system/ssh/service"))
                        (action_button(ButtonVariant::Danger, "Disable SSH", "disable-ssh", "/api/system/ssh/service"))
                        (action_button(ButtonVariant::Secondary, "Enable SSH Password", "enable-ssh-password", "/api/system/ssh/password-auth"))
                        (action_button(ButtonVariant::Danger, "Disable SSH Password", "disable-ssh-password", "/api/system/ssh/password-auth"))
                        (copy_button("Copy SSH Command", &ssh.command))
                    }
                    form id="ssh-key-form" class="settings-form system-compact-form" autocomplete="off" {
                        label { span { "Authorized public key" } textarea class="field field--textarea" name="public_key" rows="3" placeholder="ssh-ed25519 AAAA... homeconsole" {} }
                        div class="inline-actions" { button class="btn btn--primary" type="submit" { "Install Public Key" } }
                        div id="ssh-key-message" class="message" hidden {}
                    }
                }

                article class="system-card system-card--trust" {
                    div class="system-card-band" { strong { "Trust & HTTPS" } b class=(status_class(if trust.mode == "https" { "running" } else { "stopped" })) { @if trust.mode == "https" { "HTTPS" } @else { "HTTP" } } }
                    div class="system-field-grid" {
                        (system_field("Mode", if trust.mode == "https" { "HTTPS with Home Root CA" } else { "HTTP" }))
                        (system_field("Root CA", if trust.ca_installed { "Installed" } else { "Not installed" }))
                        (system_field("CA path", trust.ca_path))
                        @if let Some(subject) = trust.ca_subject.as_deref() { (system_field("Subject", subject)) }
                        @if let Some(issuer) = trust.ca_issuer.as_deref() { (system_field("Issuer", issuer)) }
                        @if let Some(expiry) = trust.ca_not_after.as_deref() { (system_field("Expires", expiry)) }
                    }
                    form id="root-ca-form" class="settings-form system-compact-form" autocomplete="off" {
                        label { span { "Root CA bundle" } textarea class="field field--textarea" name="ca_bundle" rows="5" placeholder="-----BEGIN CERTIFICATE-----" {} }
                        div class="inline-actions" { button class="btn btn--primary" type="submit" { "Install Root CA" } }
                        div id="root-ca-message" class="message" hidden {}
                    }
                    div class="inline-actions" {
                        (action_button(ButtonVariant::Secondary, "HTTP Mode", "trust-mode-http", "/api/system/trust/mode"))
                        (action_button(ButtonVariant::Primary, "HTTPS with Home Root CA", "trust-mode-https", "/api/system/trust/mode"))
                    }
                }

                article class="system-card system-card--services" {
                    div class="system-service-list" {
                        @for svc in &status.system.services {
                            (system_service_row_dynamic(&svc.name, &svc.state, &svc.detail, svc.action.as_deref().zip(svc.endpoint.as_deref())))
                        }
                    }
                }

                article class="system-card system-card--logs system-card--desktop-detail" {
                    (system_log_group("Sync"))
                    (system_log_group("Local AI"))
                    (system_log_group("Local AI Inference"))
                    (system_log_group("System"))
                    (system_log_group("Web GUI"))
                }
            }
        },
    )
}

fn status_card(title: &str, value: &str, help: &str) -> Markup {
    html! { article class="status-card" { span { (title) } strong { (value) } p { (help) } } }
}

fn sync_detail(label: &str, value: &str) -> Markup {
    html! { span class="sync-detail" { em { (label) } strong { (value) } } }
}

fn storage_stat(label: &str, value: &str) -> Markup {
    html! { div class="storage-stat" { span { (label) } strong { (value) } } }
}

fn model_meta(label: &str, value: &str) -> Markup {
    html! { span class="model-meta" { em { (label) } strong { (value) } } }
}

fn system_field(label: &str, value: &str) -> Markup {
    html! { span class="system-field" { em { (label) } strong { (value) } } }
}

fn command_box(label: &str, value: &str) -> Markup {
    html! {
        div class="command-box" {
            span { (label) }
            code { (value) }
            (copy_button("Copy", value))
        }
    }
}

fn system_power_panel() -> Markup {
    html! {
        section class="system-power-panel" aria-label="Power and sessions" {
            div class="system-power-state" {
                span class="system-power-icon" aria-hidden="true" { "⏻" }
                span { "Power & Sessions" }
                strong { "Administration" }
            }
            div class="system-power-actions" {
                (system_power_action("Restart Console", "Full system reboot", ButtonVariant::Danger, "reboot-console", "/api/actions/reboot-console"))
                (system_power_action("Shut Down", "Power off appliance", ButtonVariant::Danger, "shutdown-console", "/api/actions/shutdown-console"))
                (system_power_action("Restart Arcadia", "Web GUI only", ButtonVariant::Secondary, "restart-arcadia", "/api/actions/restart-arcadia"))
                (system_power_action("Restart GameScope", "Game session only", ButtonVariant::Secondary, "restart-gamescope", "/api/actions/restart-gamescope"))
            }
        }
    }
}

fn system_power_action(
    label: &str,
    detail: &str,
    variant: ButtonVariant,
    action: &str,
    endpoint: &str,
) -> Markup {
    html! {
        article class="system-power-action" {
            span { (detail) }
            (action_button(variant, label, action, endpoint))
        }
    }
}

fn copy_button(label: &str, value: &str) -> Markup {
    html! { button class="btn btn--secondary" type="button" data-copy-value=(value) { (label) } }
}

fn system_service_row_dynamic(
    name: &str,
    status: &str,
    detail: &str,
    restart: Option<(&str, &str)>,
) -> Markup {
    html! {
        div class="system-service-row" {
            span class="system-service-main" { strong { (name) } em { (detail) } }
            b class=(status_class(status)) { (title_case_state(status)) }
            span class="system-row-actions" {
                @if let Some((action, endpoint)) = restart {
                    (action_button(ButtonVariant::Secondary, "Restart", action, endpoint))
                }
            }
        }
    }
}

fn status_class(status: &str) -> String {
    let class = match status {
        "running" | "enabled" | "available" => "running",
        "starting" => "starting",
        "error" | "failed" => "error",
        _ => "stopped",
    };
    format!("system-status system-status--{}", class)
}

fn system_log_group(name: &str) -> Markup {
    html! {
        details class="collapsible-log system-log-group" {
            summary { (name) }
            div class="inline-actions" {
                (modal_button(ButtonVariant::Secondary, "View", name, ""))
                (copy_button("Copy", ""))
                (modal_button(ButtonVariant::Secondary, "Download", name, ""))
            }
            pre { code {} }
        }
    }
}

fn nav_focus_button(label: &str, target: &str, focus: &str) -> Markup {
    html! { button class="btn btn--secondary" type="button" data-nav-target=(target) data-focus-target=(focus) { (label) } }
}

fn action_button(variant: ButtonVariant, label: &str, action: &str, endpoint: &str) -> Markup {
    html! { button class=(format!("btn btn--{}", variant.class())) type="button" data-button=(variant.class()) data-action=(action) data-endpoint=(endpoint) { (label) } }
}

fn nav_button(label: &str, view: &str) -> Markup {
    html! { button class="btn btn--secondary" type="button" data-nav-target=(view) { (label) } }
}

fn modal_button(variant: ButtonVariant, label: &str, title: &str, body: &str) -> Markup {
    html! { button class=(format!("btn btn--{}", variant.class())) type="button" data-button=(variant.class()) data-modal-title=(title) data-modal-body=(body) { (label) } }
}

fn gui_pin_gate(status: &ConsoleStatus) -> Markup {
    html! {
        section id="gui-pin-gate" class="pin-auth-container" data-required=(status.gui_pin.pin_required) {
            article class="pin-auth-card" {
                div class="product-mark" { "H" }
                h1 { "HomeConsole" }
                h2 { "GUI PIN" }
                p { "Enter the setup PIN printed on the device card to manage this console." }
                form id="gui-pin-unlock-form" class="settings-form" autocomplete="off" {
                    input class="field field--pin" type="password" name="pin" placeholder="Enter GUI PIN" autocomplete="current-password" autofocus;
                    div id="gui-pin-auth-error" class="message message--error" hidden {}
                    button class="btn btn--primary" type="submit" { "Open Arcadia" }
                }
            }
        }
    }
}

fn theme_boot_script() -> PreEscaped<&'static str> {
    PreEscaped(
        r#"(function(){try{document.documentElement.dataset.theme=localStorage.getItem('arcadia-theme')||'ember-aubergine';}catch(_){document.documentElement.dataset.theme='ember-aubergine';}})();"#,
    )
}

fn modal_root() -> Markup {
    html! {
        div id="popup-root" data-popup-root="true" {
            div id="modal-overlay" class="modal-overlay" hidden {
                section class="modal-card" role="dialog" aria-modal="true" aria-labelledby="modal-title" {
                    button id="modal-close" class="modal-close" type="button" aria-label="Close modal" { "×" }
                    h2 id="modal-title" class="modal-title" {}
                    div id="modal-content" class="modal-content" {}
                    footer class="modal-actions" { button class="btn btn--primary" type="button" data-action="modal-ok" { "OK" } }
                }
            }
            div id="toast-container" class="toast-container" aria-live="polite" {}
        }
    }
}
