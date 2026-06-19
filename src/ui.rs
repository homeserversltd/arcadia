use maud::{html, Markup, PreEscaped, DOCTYPE};

use crate::{
    AIModelStorage, ButtonVariant, ConsoleStatus, FolderRoot, FolderStorage, GameFolderStorage,
    GameRoot,
};

const FOLDERS: [&str; 12] = [
    "gba", "genesis", "snes", "nes", "ps1", "n64", "ps2", "sega-cd", "psp", "gamecube", "wii",
    "dos",
];

const VIEWS: [(&str, &str, &str); 10] = [
    ("home", "⌂", "Home"),
    ("games", "▣", "Games"),
    ("sync", "↻", "Sync"),
    ("storage", "▰", "Storage"),
    ("local-ai", "◉", "Local AI"),
    ("network", "◌", "Network"),
    ("access-pin", "●", "Access / PIN"),
    ("updates", "⬆", "Updates"),
    ("power", "⏻", "Power"),
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
                            (games_view(status))
                            (sync_view(status))
                            (storage_view(status))
                            (ai_model_view(status))
                            (network_view(status))
                            (access_pin_view(status))
                            (updates_view(status))
                            (power_view())
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
    let network_icon = if status.network.internet_reachable == Some(true) {
        "◉"
    } else if status.network.online {
        "⌁"
    } else {
        "⊘"
    };
    let network_tip = network_tooltip(status);
    let gamescope_class = if status.arcadia.service == "running" {
        "good"
    } else if status.arcadia.service == "stopped" {
        "idle"
    } else {
        "warn"
    };
    let sync_delta = status.library.unsynced_added
        + status.library.unsynced_changed
        + status.library.unsynced_removed;
    let sync_class = if status.library.last_sync_state == "error" {
        "bad"
    } else if sync_delta > 0 {
        "warn"
    } else {
        "idle"
    };
    let sync_tip = if status.library.sync_state == "unknown" {
        "Sync state unknown".to_string()
    } else if sync_delta > 0 {
        format!("{} changes waiting for sync", sync_delta)
    } else {
        "Synced · no changes".to_string()
    };
    let updates_class = match status.updates.state.as_str() {
        "available" => "warn",
        "checking" | "installing" => "warn",
        "error" => "bad",
        "unknown" => "idle",
        _ => "idle",
    };
    let updates_tip = match status.updates.state.as_str() {
        "available" => format!(
            "Update available · {}",
            status
                .updates
                .available_version
                .as_deref()
                .unwrap_or("unknown")
        ),
        "current" => format!("Software current · {}", status.updates.current_version),
        "checking" => "Checking for updates".to_string(),
        "installing" => "Installing update".to_string(),
        "error" => "Update failed".to_string(),
        _ => "Update status unknown".to_string(),
    };
    let ai_class = match status.local_ai.load_state.as_str() {
        "hot" => "good",
        "cold" | "loading" => "warn",
        "error" => "bad",
        _ => "idle",
    };
    let ai_tip = match status.local_ai.load_state.as_str() {
        "hot" => format!(
            "{} hot-loaded",
            status
                .local_ai
                .loaded_model_name
                .as_deref()
                .unwrap_or("Local AI")
        ),
        "cold" => "Model selected but cold".to_string(),
        "loading" => "Local AI loading".to_string(),
        "error" => "Local AI error".to_string(),
        "unknown" => "Local AI state unknown".to_string(),
        _ => "No Local AI loaded".to_string(),
    };
    html! {
        header class="top-header" {
            div class="product-lockup" {
                div class="product-mark" { "A" }
                div { h1 { "Arcadia Console" } p { (status.canonical_url.trim_end_matches('/')) } }
            }
            div class="header-indicators" aria-label="Console state" {
                (nav_status_icon("connectivity", network_icon, if status.network.online { "good" } else { "bad" }, &network_tip, "network"))
                (nav_status_icon("gamescope", if status.arcadia.service == "running" { "▶" } else { "▮" }, gamescope_class, &format!("GameScope {}", status.arcadia.service), "system"))
                (nav_storage_icon(status))
                (nav_status_icon("sync", if sync_delta > 0 { "↻" } else { "✓" }, sync_class, &sync_tip, "sync"))
                (nav_status_icon("updates", if status.updates.state == "available" { "⬇" } else { "✓" }, updates_class, &updates_tip, "updates"))
                (nav_status_icon("local-ai", "◉", ai_class, &ai_tip, "local-ai"))
                (nav_status_icon("pin", if status.gui_pin.pin_required { "🔒" } else { "🔓" }, if status.gui_pin.pin_required { "warn" } else { "idle" }, if status.gui_pin.pin_required { "PIN required for GUI access" } else { "GUI open without PIN" }, "access-pin"))
            }
        }
    }
}

fn nav_status_icon(kind: &str, icon: &str, class: &str, help: &str, target: &str) -> Markup {
    html! {
        button class=(format!("status-badge status-badge--{} status-badge--nav", class)) type="button" data-nav-target=(target) data-chip-kind=(kind) title=(help) aria-label=(help) {
            span class="chip-icon" aria-hidden="true" { (icon) }
        }
    }
}

fn nav_storage_icon(status: &ConsoleStatus) -> Markup {
    html! {
        button class=(format!("status-badge status-badge--{} status-badge--nav status-badge--storage", status.storage.header_class)) type="button" data-nav-target="storage" data-chip-kind="storage" title=(status.storage.header_tooltip) aria-label=(status.storage.header_tooltip) {
            span class="chip-icon" aria-hidden="true" { "▰" }
            span class="header-microbar" aria-hidden="true" { span style=(format!("width: {}%", status.storage.percent_used)) {} }
            small { (status.storage.free) }
        }
    }
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
                (home_library_card(status))
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
    let (state, detail, action, target, endpoint): (
        String,
        String,
        Option<&str>,
        Option<&str>,
        Option<&str>,
    ) = if !status.network.online {
        (
            "Network offline".to_string(),
            "Console and game folders are unreachable.".to_string(),
            Some("Connect Wi-Fi"),
            Some("network"),
            None,
        )
    } else if status.storage.percent_used >= 90 {
        (
            format!("Storage low: {} free", status.storage.free),
            format!("{} used", status.storage.percent),
            Some("Open Storage"),
            Some("storage"),
            None,
        )
    } else if status.library.last_sync_state == "error" {
        (
            "Sync failed".to_string(),
            "Open Sync for the latest receipt.".to_string(),
            Some("View Sync"),
            Some("sync"),
            None,
        )
    } else if status.library.sync_needed {
        let changes = status.library.unsynced_added
            + status.library.unsynced_changed
            + status.library.unsynced_removed;
        (
            format!("{} changes waiting for sync", changes),
            format!(
                "{} detected · {} synced",
                status.library.total_detected_games, status.library.total_synced_entries
            ),
            Some("Start Sync"),
            None,
            Some("/api/actions/sync-games"),
        )
    } else if status.updates.state == "available" {
        (
            "Update available".to_string(),
            status
                .updates
                .available_version
                .clone()
                .unwrap_or_else(|| "Review update".to_string()),
            Some("Review Update"),
            Some("updates"),
            None,
        )
    } else if status.local_ai.load_state == "error" {
        (
            "Local AI error".to_string(),
            status
                .local_ai
                .selected_model_name
                .clone()
                .unwrap_or_else(|| "Model load failed".to_string()),
            Some("Open Local AI"),
            Some("local-ai"),
            None,
        )
    } else {
        (
            "Ready".to_string(),
            "No action needed.".to_string(),
            None,
            None,
            None,
        )
    };
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
            p class="card-line" { (status.storage.percent) " used · low-space threshold 90%" }
            div class="storage-mini-rows" {
                (storage_mini_row("Games", &status.storage.games.size, status.storage.games.percent_of_total, status.storage.games.bytes))
                (storage_mini_row("Artwork", &status.storage.artwork.size, status.storage.artwork.percent_of_total, status.storage.artwork.bytes))
                (storage_mini_row("AI Models", &status.storage.ai_models.size, status.storage.ai_models.percent_of_total, status.storage.ai_models.bytes))
                (storage_mini_row("Other", &status.storage.other.size, status.storage.other.percent_of_total, status.storage.other.bytes))
            }
            div class="inline-actions inline-actions--compact" {
                (nav_button("Open Storage", "storage"))
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
                    (reachability("Console", status.network.console_reachable))
                    (reachability("Folders", status.network.samba_reachable))
                    @if let Some(internet) = status.network.internet_reachable { (reachability("Internet", internet)) }
                    (reachability("LAN AI", status.network.lan_ai_reachable))
                }
                div class="inline-actions inline-actions--compact" {
                    (nav_focus_button("Manage Wi-Fi", "network", "wifi-management"))
                    @if let Some(path) = status.surfaces.windows_unc.as_deref().or(status.surfaces.smb_url.as_deref()) { (copy_button("Copy folders", path)) } @else { button class="btn btn--secondary" type="button" disabled { "Game folders unavailable" } }
                    (copy_button("Copy console URL", status.identity.web_origin.as_str()))
                }
            } @else {
                h3 { "Network offline" }
                p class="card-line" { "Console and game folders are unreachable." }
                (nav_button("Connect Wi-Fi", "network"))
            }
        }
    }
}

fn home_library_card(status: &ConsoleStatus) -> Markup {
    html! {
        article class="operational-card library-home-card" {
            div class="card-head" { h3 { "Game Library" } strong { (status.library.total_detected_games) " game" (if status.library.total_detected_games == 1 { "" } else { "s" }) " · " (if status.library.sync_needed { "changes" } else if status.library.sync_state == "unknown" { "unknown" } else { "synced" }) } }
            div class="state-rows state-rows--compact" {
                (state_row("Detected", &status.library.total_detected_games.to_string()))
                (state_row("Synced", &status.library.total_synced_entries.to_string()))
                (state_row("Last sync", &status.library.last_sync))
                (state_row("Artwork", &status.library.artwork_status))
            }
            div class="inline-actions inline-actions--compact" {
                (nav_button("View Details", "sync"))
            }
        }
    }
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
    let gpu = status.local_ai.gpu_memory.as_deref().unwrap_or("—");
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
                (state_row("GPU", gpu))
                (state_row("LAN", &lan))
            }
            @if let (Some(used), Some(total)) = (status.local_ai.gpu_memory_used_bytes, status.local_ai.gpu_memory_total_bytes) {
                div class="gpu-bar" aria-label="GPU memory usage" { span style=(format!("width: {}%", ((used.saturating_mul(100) / total.max(1)).min(100)))) {} }
            }
            div class="inline-actions inline-actions--compact" {
                @if status.local_ai.load_state == "hot" || status.local_ai.load_state == "loading" { (modal_button(ButtonVariant::Secondary, "Unload", "Unload local AI", "Unload the active local AI model when the backend control is connected.")) }
                @else if !status.local_ai.available_models.is_empty() { (modal_button(ButtonVariant::Primary, "Load", "Load local AI", "Load the selected local AI model when the backend control is connected.")) }
                (nav_button("Open Local AI", "local-ai"))
                (nav_button("Open Local AI", "local-ai"))
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

fn games_view(status: &ConsoleStatus) -> Markup {
    view_shell(
        "games",
        "Network copy",
        "Add Games",
        "Copy games to the console over your home network. The console stores them locally.",
        html! {
            div class="path-grid" {
                (path_card("Windows", "\\\\HOMECONSOLE"))
                (path_card("Linux / macOS", status.surfaces.smb_url.as_deref().unwrap_or("Folder address unavailable")))
            }
            div class="folder-card" {
                h3 { "Game folders" }
                div class="folder-list" {
                    @for folder in FOLDERS { code { (folder) } }
                }
            }
            article class="instruction-card" {
                h3 { "Add games in four steps" }
                ol class="numbered-list" {
                    li { "Open the network share." }
                    li { "Copy games into the matching folder." }
                    li { "Return here and press Sync Games." }
                    li { "Games appear in the GameScope library after sync." }
                }
                p class="note" { "Do not rename system folders. Large copies may take time before sync sees the files." }
            }
            div class="primary-actions" {
                (link_button(ButtonVariant::Primary, "Open Games Folder", "open-games-folder", status.surfaces.smb_url.as_deref().unwrap_or("#")))
                (nav_button("Go to Sync", "sync"))
            }
        },
    )
}

fn sync_view(status: &ConsoleStatus) -> Markup {
    let storage_blocked = status.storage.health == "Full";
    let storage_low = status.storage.percent_used >= 90;
    view_shell("sync", "Library transformation", "Sync Games", "Sync scans the console’s game folders, finds copied games, fetches artwork, and adds playable entries to GameScope.", html! {
        section class="sync-command-center" data-sync-root="true" data-storage-health=(status.storage.health) data-storage-low=(storage_low) data-storage-blocked=(storage_blocked) aria-labelledby="sync-command-title" {
            div class="sync-command-copy" {
                h3 id="sync-command-title" { "Turn copied files into playable games" }
                p { "I copy games into folders. Sync turns those files into a usable console library." }
                div class="sync-state-line" {
                    span { "Current state" }
                    strong id="sync-state" data-sync-state="idle" { "Waiting" }
                }
            }
            div class="sync-primary-action" {
                (action_button(ButtonVariant::Primary, if storage_blocked { "Storage Full" } else { "Start Sync" }, "sync-games", "/api/actions/sync-games"))
                p id="sync-progress-text" { (if storage_blocked { "Storage is full. Free space before syncing games." } else { "Ready to scan copied games." }) }
            }
        }

        @if storage_blocked {
            div class="warning sync-storage-warning" {
                strong { "Storage is full. Free space before syncing games." }
                (nav_button("Open Storage", "storage"))
            }
        } @else if storage_low {
            div class="warning sync-storage-warning" {
                strong { "Storage is low. Sync may fail if there is not enough space for artwork or library entries." }
                (nav_button("Open Storage", "storage"))
            }
        }

        div class="sync-state-legend" aria-label="Sync step states" {
            span { "Waiting" }
            span { "Running" }
            span { "Complete" }
            span { "Skipped" }
            span { "Error" }
        }
        section class="sync-workflow" aria-label="Sync workflow" {
            (sync_step("1", "▣", "Copy Games", "Copy game files into the matching console folders over the network.", "Waiting", html! {
                (link_button(ButtonVariant::Secondary, "Open Games Folder", "open-games-folder", status.surfaces.smb_url.as_deref().unwrap_or("#")))
                (nav_button("View Add Games Instructions", "games"))
            }))
            (sync_step("2", "⌕", "Scan Library", "The console scans game folders and detects new, changed, or removed files.", "Waiting", html! {
                (sync_detail("Games found", "Not run yet"))
                (sync_detail("New games", "Not run yet"))
                (sync_detail("Removed games", "Not run yet"))
                (sync_detail("Changed files", "Not run yet"))
            }))
            (sync_step("3", "★", "Fetch Artwork", "Optional metadata providers improve titles, covers, and artwork.", "Waiting", html! {
                (sync_detail("Artwork found", "Not run yet"))
                (sync_detail("Artwork missing", "Not run yet"))
                (sync_detail("Provider key status", "Optional"))
                (sync_detail("SteamGridDB", "Missing"))
                (sync_detail("TheGamesDB", "Missing"))
                (sync_detail("ScreenScraper", "Missing"))
                p class="sync-small-copy" { "Games still work without artwork keys." }
                a class="sync-inline-link" href="#sync-provider-settings" { "Configure Metadata Providers" }
            }))
            (sync_step("4", "＋", "Create Game Entries", "The console creates or updates GameScope library entries for detected games.", "Waiting", html! {
                (sync_detail("Entries created", "Not run yet"))
                (sync_detail("Entries updated", "Not run yet"))
                (sync_detail("Entries skipped", "Not run yet"))
                (sync_detail("Errors", "Not run yet"))
            }))
            (sync_step("5", "▶", "Available in GameScope", "Synced games appear in the GameScope library after sync completes.", "Waiting", html! {
                (sync_detail("Last successful sync", "Not reported"))
                (sync_detail("Total synced games", "Not reported"))
                (sync_detail("GameScope state", "Running"))
                p class="sync-small-copy" { "GameScope is running. Synced games should appear after sync completes." }
            }))
        }

        section class="sync-result-card" aria-labelledby="sync-result-title" data-sync-result="waiting" {
            div class="section-heading section-heading--compact" {
                h3 id="sync-result-title" { "Result Summary" }
                p id="sync-result-copy" { "Start Sync to scan copied games and create playable GameScope entries." }
            }
            div class="sync-result-grid" {
                (sync_detail("Games found", "Not run yet"))
                (sync_detail("New entries created", "Not run yet"))
                (sync_detail("Entries updated", "Not run yet"))
                (sync_detail("Artwork downloaded", "Not run yet"))
                (sync_detail("Artwork missing", "Not run yet"))
                (sync_detail("Errors", "Not run yet"))
                (sync_detail("Duration", "Not run yet"))
                (sync_detail("Completed time", "Not run yet"))
            }
        }

        div class="sync-secondary-actions" {
            (link_button(ButtonVariant::Secondary, "Open Games Folder", "open-games-folder", status.surfaces.smb_url.as_deref().unwrap_or("#")))
            a class="btn btn--secondary" href="#sync-provider-settings" { "Configure Metadata Providers" }
            a class="btn btn--secondary" href="#sync-output-panel" { "Output" }
            (nav_button("Open Storage", "storage"))
        }

        details id="sync-provider-settings" class="settings-panel sync-provider-panel" {
            summary { "Configure Metadata Providers" }
            p { "Metadata keys are optional. They improve artwork and titles, but games can still sync without them." }
            div class="provider-status-grid" {
                (provider_status("SteamGridDB", "Missing"))
                (provider_status("TheGamesDB", "Missing"))
                (provider_status("ScreenScraper", "Missing"))
            }
            form id="provider-keys-form" class="settings-form" autocomplete="off" {
                label { span { "SteamGridDB API key" } input class="field" type="password" name="steamgriddb_api_key" autocomplete="off"; }
                label { span { "TheGamesDB API key" } input class="field" type="password" name="thegamesdb_api_key" autocomplete="off"; }
                label { span { "ScreenScraper API key" } input class="field" type="password" name="screenscraper_api_key" autocomplete="off"; }
                div id="provider-keys-message" class="message" hidden {}
                button class="btn btn--primary" type="submit" { "Save Optional Keys" }
            }
        }
        details id="sync-output-panel" class="collapsible-log" {
            summary { "Output" }
            pre { code id="sync-output" {} }
        }
        div id="console-action-message" class="message" hidden {}
    })
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
    let diagnostics_count = status.storage.diagnostics.missing_dirs.len()
        + status.storage.diagnostics.permission_errors.len()
        + status.storage.diagnostics.warnings.len()
        + status.storage.diagnostics.overlap_warnings.len()
        + status.storage.diagnostics.category_scan_errors.len();
    view_shell(
        "storage",
        "",
        "",
        "",
        html! {
            section class="storage-appliance" aria-label="Storage overview" {
                article class="storage-summary storage-summary--compact" {
                    div class="storage-overview-line" {
                        div {
                            strong { "Storage " (status.storage.health) }
                            span { (status.storage.free) " free · " (status.storage.percent) " used" }
                        }
                        (action_button(ButtonVariant::Secondary, "Rescan", "storage-rescan", "/api/storage/rescan-summary"))
                    }
                    div class="storage-hero-stats storage-hero-stats--compact" {
                        (storage_stat("Total", &status.storage.total))
                        (storage_stat("Used", &status.storage.used))
                        (storage_stat("Scanned", &human_scan_time(&status.storage.scanned_at)))
                    }
                    (storage_usage_bar(status))
                    @if diagnostics_count > 0 {
                        div class="storage-alert-line" { strong { "Storage mismatch detected" } button class="btn btn--secondary" type="button" data-storage-modal="diagnostics" { "Review" } }
                    } @else if status.storage.percent_used >= status.storage.thresholds.low_percent {
                        div class="storage-alert-line" { strong { "Storage low" } button class="btn btn--secondary" type="button" data-storage-modal="cleanup-review" { "Review Cleanup" } }
                    }
                }

                div class="storage-category-list" aria-label="Storage categories" {
                    (storage_category_row("🎮", "Games", &status.storage.games.size, status.storage.games.percent_of_total, "games", "games", None, None, true))
                    (storage_category_row("🖼", "Artwork", &status.storage.artwork.size, status.storage.artwork.percent_of_total, "artwork", "artwork-detail", Some("Clear"), Some("clear-artwork-cache"), status.storage.artwork.bytes > 0))
                    (storage_category_row("◉", "AI Models", &status.storage.ai_models.size, status.storage.ai_models.percent_of_total, "ai", "ai-models-detail", None, None, true))
                    (storage_category_row("⬇", "Updates", &status.storage.categories.updates.size, status.storage.categories.updates.percent_of_total, "updates", "updates-detail", Some("Clean"), Some("clear-old-updates"), status.storage.categories.updates.bytes > 0))
                    (storage_category_row("≋", "Logs", &status.storage.categories.logs.size, status.storage.categories.logs.percent_of_total, "logs", "logs-detail", Some("Prune"), Some("prune-logs"), status.storage.categories.logs.bytes > 0))
                    (storage_category_row("⌁", "Temporary Files", &status.storage.categories.temporary.size, status.storage.categories.temporary.percent_of_total, "temporary", "temporary-detail", Some("Clean"), Some("clean-temporary-files"), status.storage.categories.temporary.bytes > 0))
                    (storage_category_row("▣", "System", &status.storage.categories.system.size, status.storage.categories.system.percent_of_total, "system", "system-detail", None, None, status.storage.categories.system.bytes > 0))
                    (storage_category_row("◇", "Other", &status.storage.other.size, status.storage.other.percent_of_total, "other", "category-other", None, None, status.storage.other.bytes > 0))
                    (storage_category_row("○", "Free", &status.storage.free, 100u8.saturating_sub(status.storage.percent_used), "free", "category-free", None, None, true))
                }

                article class="storage-compact-entry" {
                    span { "Cleanup available: " (human_or_zero(cleanup_total)) }
                    button class="btn btn--secondary" type="button" data-storage-modal="cleanup-review" { "Review Cleanup" }
                }
                article class="storage-compact-entry" {
                    span { "Managed Locations" }
                    small { "Games, artwork, AI models, updates, logs" }
                    button class="btn btn--secondary" type="button" data-storage-modal="locations" { "Open" }
                }
                article class="storage-compact-entry storage-compact-entry--diagnostics" {
                    @if diagnostics_count > 0 {
                        span { "Storage scan warnings: " (diagnostics_count) }
                        button class="btn btn--secondary" type="button" data-storage-modal="diagnostics" { "Review" }
                    } @else {
                        span { "Diagnostics" }
                        button class="btn btn--secondary" type="button" data-storage-modal="diagnostics" { "Open" }
                    }
                }
            }
        },
    )
}

fn storage_category_row(
    icon: &str,
    label: &str,
    size: &str,
    percent: u8,
    color: &str,
    modal: &str,
    cleanup_label: Option<&str>,
    cleanup_action: Option<&str>,
    show: bool,
) -> Markup {
    if !show {
        return html! {};
    }
    html! {
        article class="storage-category-row" data-storage-category=(modal) {
            span class="storage-category-icon" { (icon) }
            strong { (label) }
            b { (size) }
            em { (percent_label(percent, if size == "0 B" { 0 } else { 1 })) }
            div class="storage-category-mini" aria-hidden="true" { span class=(format!("storage-segment--{}", color)) style=(format!("width: {}%", percent.max(if size == "0 B" { 0 } else { 1 }))) {} }
            button class="btn btn--secondary" type="button" data-storage-modal=(modal) { "Details" }
            @if let (Some(text), Some(action)) = (cleanup_label, cleanup_action) {
                button class="btn btn--secondary" type="button" data-storage-cleanup=(action) { (text) }
            }
        }
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

fn game_folder_row(folder: &GameFolderStorage) -> Markup {
    html! { details class="storage-table-row storage-folder-row" {
        summary { span { (folder.platform) } span { (folder.display_name) } span { (folder.size) } span { (folder.file_count) } span { (folder.synced_entries.map(|v| v.to_string()).unwrap_or_else(|| "Unknown".to_string())) } span { (folder.unsynced_files.map(|v| v.to_string()).unwrap_or_else(|| "Unknown".to_string())) } span class="system-row-actions" { (folder_copy_button("Copy path", folder)) (nav_button("View in Sync", "sync")) } }
        div class="folder-detail-grid" {
            (system_field("Filesystem path", &folder.path))
            (system_field("Share", &folder.samba_share_name))
            @if folder.largest_files.is_empty() { (system_field("Largest files", "None")) }
            @else { @for file in &folder.largest_files { (system_field(&file.name, &file.size)) } }
        }
    } }
}

fn folder_store_row(store: &FolderStorage) -> Markup {
    html! { div class="network-row" { span { strong { (store.display_name) } em { (store.path) } } b class=(format!("system-status system-status--{}", store.state)) { (store.state) } span { (store.size) } span { (store.file_count) " files" } } }
}
fn ai_model_file_row(model: &AIModelStorage) -> Markup {
    html! { details class="storage-model-row" { summary { span { strong { (model.name) } code { (model.filename) } } em { (model.size) } b { (if model.loaded { "Hot" } else if model.selected { "Selected" } else { "Installed" }) } @if model.removable { (action_button(ButtonVariant::Danger, "Remove Model", "remove-ai-model", &format!("/api/actions/remove-ai-model?name={}", model.filename))) } @else { button class="btn btn--secondary" type="button" disabled { "Unload first" } } } code { (model.path) } } }
}
fn cleanup_card(
    label: &str,
    bytes: u64,
    action_label: &str,
    action: &str,
    endpoint: &str,
) -> Markup {
    html! { article class="cleanup-card" { span { (label) } strong { (human_or_zero(bytes)) } @if bytes > 0 { (action_button(ButtonVariant::Secondary, action_label, action, endpoint)) } @else { button class="btn btn--secondary" type="button" disabled { "—" } } } }
}
fn game_location_row(root: &GameRoot) -> Markup {
    html! { div class="location-row" { span { strong { (root.display_name) } code { (root.path) } small { "Share: " (root.windows_unc.as_deref().unwrap_or(&root.samba_share_name)) } } (copy_button("Copy", &root.path)) } }
}
fn folder_location_row(root: &FolderRoot) -> Markup {
    html! { div class="location-row" { span { strong { (root.display_name) } code { (root.path) } } (copy_button("Copy", &root.path)) } }
}
fn folder_copy_button(label: &str, folder: &GameFolderStorage) -> Markup {
    html! { button class="btn btn--secondary" type="button" data-folder-copy="true" data-windows=(folder.windows_unc.as_deref().unwrap_or("")) data-windows-ip=(folder.windows_unc_by_ip.as_deref().unwrap_or("")) data-smb=(folder.smb_url.as_deref().unwrap_or("")) data-smb-ip=(folder.smb_url_by_ip.as_deref().unwrap_or("")) { (label) } }
}
fn human_or_zero(bytes: u64) -> String {
    if bytes == 0 {
        "0 B".to_string()
    } else {
        crate::human_size(bytes)
    }
}

fn ai_model_view(status: &ConsoleStatus) -> Markup {
    let loaded_name = status
        .local_ai
        .loaded_model_name
        .as_deref()
        .or(status.local_ai.selected_model_name.as_deref())
        .unwrap_or("No model loaded");
    let endpoint = format!("{}:{}", status.identity.web_origin, 7777);
    let inference_on = status.local_ai.lan_inference_enabled;
    view_shell(
        "local-ai",
        "",
        "",
        "",
        html! {
            section class="local-ai-section ai-manager-section" aria-label="Runtime" {
                div class="network-section-head" { h3 { "Runtime" } }
                div class="system-field-grid" {
                    (system_field("Runtime", "llama.cpp"))
                    (system_field("Installed version", "Detected when installed"))
                    (system_field("Status", if status.local_ai.load_state == "error" { "Error" } else { "Ready" }))
                    (system_field("Server", if inference_on { "Running" } else { "Stopped" }))
                }
                div class="inline-actions inline-actions--compact" {
                    button class="btn btn--secondary" type="button" data-ai-action="runtime-check-update" { "Check for Runtime Update" }
                    button class="btn btn--secondary" type="button" data-ai-action="runtime-update" { "Update llama.cpp" }
                    button class="btn btn--secondary" type="button" data-ai-action="runtime-restart" { "Restart Runtime" }
                }
            }

            section class="local-ai-section ai-manager-section" aria-label="Loaded Model" {
                div class="network-section-head" { h3 { "Loaded Model" } }
                div class=(if status.local_ai.load_state == "hot" { "active-model active-model--hot" } else { "active-model" }) {
                    span { "Loaded Model" }
                    strong { (title_case_state_like(&status.local_ai.load_state)) " · " (loaded_name) }
                    @if status.local_ai.load_state == "unloaded" { p { "Select an installed model to load." } }
                    div class="model-meta-row" {
                        (model_meta("GPU", status.local_ai.gpu_memory.as_deref().unwrap_or("GPU telemetry unavailable")))
                        (model_meta("Inference", if inference_on { "On · :7777" } else { "Off" }))
                        (model_meta("State", title_case_state_like(&status.local_ai.load_state)))
                    }
                    div class="inline-actions" {
                        @if status.local_ai.available_models.is_empty() { (nav_focus_button("Get Models", "local-ai", "get-models")) }
                        @else if status.local_ai.load_state != "hot" && status.local_ai.load_state != "loading" { button class="btn btn--primary" type="button" data-ai-action="model-load" data-model-id=(status.local_ai.selected_model_id.as_deref().unwrap_or("")) { "Load" } }
                        @if status.local_ai.load_state == "hot" || status.local_ai.load_state == "loading" { button class="btn btn--secondary" type="button" data-ai-action="model-unload" { "Unload" } }
                        @if status.local_ai.load_state == "hot" || inference_on { button class="btn btn--secondary" type="button" data-ai-action="runtime-restart" { "Restart" } }
                        button class="btn btn--secondary" type="button" data-ai-logs="true" { "Open Logs" }
                    }
                }
            }

            section class="local-ai-section ai-manager-section" aria-label="Installed Models" {
                div class="network-section-head" { h3 { "Installed Models" } }
                div class="model-grid" {
                    @if status.local_ai.available_models.is_empty() {
                        article class="model-card" { strong class="model-name" { "No models installed" } span class="model-filename" { "Install Inharmonia or download a compatible model from Hugging Face." } div class="inline-actions" { (nav_focus_button("Install Inharmonia", "local-ai", "get-models")) } }
                    } @else {
                        @for model in &status.local_ai.available_models {
                            (installed_model_card(model, status))
                        }
                    }
                }
            }

            section id="get-models" class="local-ai-section ai-manager-section" aria-label="Get Models" tabindex="-1" {
                div class="network-section-head" { h3 { "Get Models" } }
                div class="model-grid" {
                    article class="model-card model-card--recommended" {
                        strong class="model-name" { "Inharmonia" }
                        span class="model-filename" { "Recommended · Balanced local assistant for Arcadia." }
                        div class="model-meta-row" { (model_meta("Use", "Balanced")) (model_meta("Source", "Product catalog")) }
                        @if status.local_ai.available_models.iter().any(|model| model.is_inharmonia) { b class="model-status" { "Installed" } }
                        @else { button class="btn btn--primary" type="button" data-ai-action="install-recommended" data-model-id="inharmonia" { "Install Inharmonia" } }
                    }
                }
                article class="form-card" data-hf-installer="true" {
                    h3 { "Hugging Face model" }
                    label { span { "Repository" } input class="field" name="repoId" placeholder="TheBloke/example-GGUF" autocomplete="off"; }
                    label { span { "File" } input class="field" name="filename" placeholder="example.Q4_K_M.gguf" autocomplete="off"; }
                    label { span { "Revision" } input class="field" name="revision" placeholder="main" autocomplete="off"; }
                    div class="inline-actions" { button class="btn btn--secondary" type="button" data-ai-action="hf-list-files" { "Fetch Files" } button class="btn btn--primary" type="button" data-ai-action="hf-download" { "Download" } }
                    div id="hf-file-results" class="diagnostics-results" {}
                }
            }

            section id="local-ai-inference" class="local-ai-section ai-manager-section" aria-label="Inference" tabindex="-1" {
                div class="network-section-head" { h3 { "Inference" } }
                div class="system-field-grid" {
                    (system_field("Local API", if inference_on { "On" } else { "Off" }))
                    (system_field("LAN Access", if inference_on { "On" } else { "Off" }))
                    (system_field("Port", "7777"))
                    (system_field("Endpoint", if inference_on { &endpoint } else { "No model loaded" }))
                }
                p class="warning" { "LAN access is for trusted home networks only. Do not expose port 7777 to the public internet." }
                div class="inline-actions inline-actions--compact" {
                    button class="btn btn--primary" type="button" data-ai-action="inference-enable" { "Enable Inference" }
                    button class="btn btn--secondary" type="button" data-ai-action="inference-disable" { "Disable Inference" }
                    button class="btn btn--secondary" type="button" data-ai-action="inference-test" { "Open API Test" }
                    @if inference_on { (copy_button("Copy Endpoint", &endpoint)) } @else { button class="btn btn--secondary" type="button" disabled { "Copy Endpoint" } }
                }
            }

            section class="local-ai-section ai-manager-section" aria-label="GPU & Storage" {
                div class="network-section-head" { h3 { "GPU & Storage" } }
                @if let (Some(used), Some(total)) = (status.local_ai.gpu_memory_used_bytes, status.local_ai.gpu_memory_total_bytes) {
                    (meter_block("GPU", &human_bytes(used), &human_bytes(total), used, total))
                } @else { div class="empty-state" { strong { "GPU telemetry unavailable" } } }
                (meter_block("AI Model Storage", &status.storage.ai_models.size, &status.storage.free, status.storage.ai_models.bytes, status.storage.total_bytes.max(1)))
                p class="warning" { "Games and Local AI share GPU resources." }
                div class="inline-actions" { (nav_button("Open Storage", "storage")) @if status.storage.ai_models.bytes > 0 { (nav_focus_button("Remove Unused Models", "local-ai", "installed-models")) } }
            }

            section class="local-ai-section ai-manager-section" aria-label="Activity" {
                div class="network-section-head" { h3 { "Activity" } }
                div id="ai-activity" class="system-field-grid" { (system_field("Current operation", if inference_on { "serving inference" } else { "idle" })) (system_field("Last error", "None")) }
                details class="collapsible-log" { summary { "Runtime update log" } pre { code { "No runtime update log reported." } } }
                details class="collapsible-log" { summary { "Model download log" } pre { code { "No model download log reported." } } }
                details class="collapsible-log" { summary { "Model load log" } pre { code { "No model load log reported." } } }
                details class="collapsible-log" { summary { "Inference server log" } pre { code { "No inference server log reported." } } }
            }
            div id="ai-message" class="message" hidden {}
        },
    )
}

fn installed_model_card(model: &crate::LocalAiModelStatus, status: &ConsoleStatus) -> Markup {
    let selected = status.local_ai.selected_model_id.as_deref() == Some(model.id.as_str());
    let hot = status.local_ai.loaded_model_id.as_deref() == Some(model.id.as_str());
    html! { article id="installed-models" class=(if selected { "model-card model-card--selected" } else { "model-card" }) {
        strong class="model-name" { (model.name) @if model.is_inharmonia { " · Recommended" } }
        span class="model-filename" { (model.filename) }
        div class="model-meta-row" {
            (model_meta("Size", &model.size))
            (model_meta("Quantization", model.quantization.as_deref().unwrap_or("Unknown")))
            (model_meta("Estimated VRAM", &model.estimated_vram_bytes.map(human_bytes).unwrap_or_else(|| "Unknown".to_string())))
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
    let folders = status
        .samba
        .shares
        .iter()
        .find(|share| share.purpose == "games");
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

                article id="wifi-management" class="network-section network-card" tabindex="-1" aria-label="Wi-Fi" {
                    div class="network-section-head" {
                        strong { "Wi-Fi" }
                        span class=(format!("system-status system-status--{}", if status.network.wifi_adapter_available { "available" } else { "disabled" })) {
                            (wifi_status_label(status))
                        }
                    }
                    div class="network-summary-lines" {
                        @if status.network.active_type == "wifi" {
                            span { (status.network.ssid.as_deref().unwrap_or("Wi-Fi")) " · " (status.network.signal_percent.map(|v| format!("{}% signal", v)).unwrap_or_else(|| "Signal unavailable".to_string())) }
                        } @else if status.network.wifi_adapter_available {
                            span { "Available for wireless setup." }
                        } @else {
                            span { "No Wi-Fi adapter was detected." }
                        }
                    }
                    div class="inline-actions inline-actions--compact" {
                        button class="btn btn--primary" type="button" data-network-action="choose-wifi" disabled[!status.network.wifi_adapter_available] { "Choose Network" }
                        button class="btn btn--secondary" type="button" data-network-action="scan-wifi" disabled[!status.network.wifi_adapter_available] { "Scan" }
                        button class="btn btn--secondary" type="button" data-open-hidden-wifi="true" disabled[!status.network.wifi_adapter_available] { "Join Hidden Network" }
                        button class="btn btn--secondary" type="button" data-network-action="wifi-toggle" data-enabled=(if status.network.active_type == "wifi" { "false" } else { "true" }) disabled[!status.network.wifi_adapter_available] { (if status.network.active_type == "wifi" { "Turn Off" } else { "Turn On" }) }
                    }
                    div id="wifi-message" class="message" hidden {}
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
                        @if let Some(share) = folders { (network_service_row("Game Folders", "Available", share.windows_unc.as_deref().or(share.smb_url.as_deref()).unwrap_or("Folder address unavailable"), html! { (folder_copy_menu_button("Copy address", share)) })) }
                        @else { (network_service_row("Game Folders", "Disabled", "Folder address unavailable", html! { (nav_button("Open System", "system")) })) }
                        (network_service_row("LAN AI", if status.network.lan_ai_reachable { "Available" } else { "Disabled" }, if status.network.lan_ai_reachable { ":7777" } else { "" }, html! { (nav_focus_button("Open Local AI", "local-ai", "local-ai-inference")) }))
                        (network_service_row("SSH", "Disabled", "", html! { (nav_button("Open System", "system")) }))
                    }
                }

            details class="network-section diagnostics-panel" {
                summary { "Diagnostics" }
                div class="inline-actions inline-actions--compact" data-diagnostics-actions="true" {
                    button class="btn btn--secondary" type="button" data-diagnostic="gateway" { "Test Gateway" }
                    button class="btn btn--secondary" type="button" data-diagnostic="dns" { "Test DNS" }
                    button class="btn btn--secondary" type="button" data-diagnostic="internet" { "Test Internet" }
                    button class="btn btn--secondary" type="button" data-diagnostic="game-folders" { "Test Game Folders" }
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

fn folder_copy_menu_button(label: &str, share: &crate::SambaShareStatus) -> Markup {
    html! { button class="btn btn--secondary" type="button" data-folder-copy="true"
    data-windows=(share.windows_unc.as_deref().unwrap_or(""))
    data-windows-ip=(share.windows_unc_by_ip.as_deref().unwrap_or(""))
    data-smb=(share.smb_url.as_deref().unwrap_or(""))
    data-smb-ip=(share.smb_url_by_ip.as_deref().unwrap_or("")) { (label) } }
}

fn access_pin_view(status: &ConsoleStatus) -> Markup {
    view_shell("access-pin", "Router-style access", "Access & PIN", "The GUI PIN protects this management console. The initial PIN is printed on the device setup card.", html! {
        div class="task-hero" {
            div { strong { "Current mode" } span { (if status.gui_pin.pin_required { "PIN Required" } else { "Open Without PIN" }) } }
            div class="inline-actions" data-module="gui-pin-access" {
                button class="btn btn--secondary" type="button" data-action="gui-pin-enable" data-endpoint="/api/gui-pin/access" data-pin-required="true" { "Require GUI PIN" }
                button class="btn btn--secondary" type="button" data-action="gui-pin-disable" data-endpoint="/api/gui-pin/access" data-pin-required="false" { "Open Without PIN" }
            }
        }
        div id="gui-pin-access-message" class="message" hidden {}
        article class="form-card" {
            h3 { "Change PIN" }
            p { "Enter the current PIN, then choose a new PIN. Arcadia never displays saved PIN values." }
            form id="gui-pin-change-form" class="settings-form" autocomplete="off" {
                label { span { "Current PIN" } input class="field" type="password" name="current_pin" autocomplete="current-password" required; }
                label { span { "New PIN" } input class="field" type="password" name="new_pin" autocomplete="new-password" required minlength="4"; }
                label { span { "Confirm New PIN" } input class="field" type="password" name="confirm_pin" autocomplete="new-password" required minlength="4"; }
                div id="gui-pin-change-message" class="message" hidden {}
                button class="btn btn--primary" type="submit" { "Change PIN" }
            }
        }
        (instruction_card("If the PIN is lost", "Use the physical recovery/reset procedure documented with the console. The saved PIN is never shown here."))
    })
}

fn updates_view(status: &ConsoleStatus) -> Markup {
    view_shell(
        "updates",
        "Software maintenance",
        "Updates",
        "Updates may restart console services. Games should be closed before updating.",
        html! {
            div class="status-card-grid" {
                (status_card("Current version", status.arcadia.version, "Installed Arcadia GUI version."))
                (status_card("Latest available", "Not checked", "Press Check for Updates to ask the console to update."))
                (status_card("Last checked", "Not reported", "The updater receipt reports the last check time."))
            }
            div class="primary-actions" {
                (action_button(ButtonVariant::Primary, "Check for Updates", "update-gui", "/api/actions/update-gui"))
                (action_button(ButtonVariant::Secondary, "Install Update", "update-gui", "/api/actions/update-gui"))
            }
            (collapsible_log("Update log", "Update receipts appear under /var/lib/harmonia/receipts/arcadia-gui-latest on the console."))
        },
    )
}

fn power_view() -> Markup {
    view_shell("power", "Safe shutdown", "Power", "Restart or shut down the appliance safely. Dangerous actions ask for confirmation before they run.", html! {
        div class="power-grid" {
            (power_action("Restart Console", "Full system reboot.", ButtonVariant::Danger, "reboot-console", "/api/actions/reboot-console"))
            (power_action("Shut Down Console", "Powers off the appliance.", ButtonVariant::Danger, "shutdown-console", "/api/actions/shutdown-console"))
            (power_action("Restart GameScope", "Restarts the game session only.", ButtonVariant::Secondary, "restart-gamescope", "/api/actions/restart-gamescope"))
        }
    })
}

fn system_view(status: &ConsoleStatus) -> Markup {
    view_shell(
        "system",
        "Machine status",
        "System",
        "",
        html! {
            section class="system-grid" aria-label="System" {
                article class="system-card system-card--ssh" {
                    div class="system-field-grid" {
                        (system_field("SSH status", "Disabled"))
                        (system_field("Hostname", "console.home.arpa"))
                        (system_field("LAN IP address", "DHCP assigned"))
                        (system_field("Username", "console"))
                    }
                    (command_box("Example command", "ssh console@console.home.arpa"))
                    div class="inline-actions" {
                        (modal_button(ButtonVariant::Secondary, "Enable SSH", "Enable SSH", "Confirm enabling SSH."))
                        (modal_button(ButtonVariant::Secondary, "Disable SSH", "Disable SSH", "Confirm disabling SSH."))
                        (copy_button("Copy SSH Command", "ssh console@console.home.arpa"))
                    }
                }

                article class="system-card system-card--services" {
                    div class="system-service-list" {
                        (system_service_row("GameScope", "Running", "Not reported", Some(("restart-gamescope", "/api/actions/restart-gamescope"))))
                        (system_service_row("Samba", "Running", "Not reported", None))
                        (system_service_row("Game Sync", "Stopped", "Runs on demand", None))
                        (system_service_row("Local AI", "Stopped", "Not reported", None))
                        (system_service_row("Local AI Inference", "Stopped", "Not reported", None))
                        (system_service_row("Web GUI", "Running", "Current", None))
                    }
                }

                article class="system-card system-card--logs" {
                    (system_log_group("Sync"))
                    (system_log_group("Local AI"))
                    (system_log_group("Local AI Inference"))
                    (system_log_group("System"))
                    (system_log_group("Web GUI"))
                }

                article class="system-card system-card--networking" {
                    div class="system-field-grid" {
                        (system_field("Hostname", "console.home.arpa"))
                        (system_field("Local domain/path", status.canonical_url.trim_end_matches('/')))
                        (system_field("LAN IP address", "DHCP assigned"))
                        (system_field("MAC address", "Not reported"))
                        (system_field("Status", "Online"))
                        (system_field("Active interface", "LAN"))
                    }
                    div class="system-endpoints" {
                        (command_box("Web GUI", "http://console.home.arpa"))
                        (command_box("Games Folder", "\\\\HOMECONSOLE"))
                        (command_box("Local AI Inference", "http://console.home.arpa:7777"))
                    }
                    div class="system-port-list" {
                        (system_field("80/443", "Web GUI"))
                        (system_field("445", "Samba"))
                        (system_field("7777", "Local AI Inference"))
                        (system_field("22", "SSH"))
                    }
                }
            }
        },
    )
}

fn status_card(title: &str, value: &str, help: &str) -> Markup {
    html! { article class="status-card" { span { (title) } strong { (value) } p { (help) } } }
}

fn instruction_card(title: &str, text: &str) -> Markup {
    html! { article class="instruction-card" { h3 { (title) } p { (text) } } }
}

fn path_card(title: &str, path: &str) -> Markup {
    html! { article class="path-card" { span { (title) } code { (path) } } }
}

fn provider_status(name: &str, state: &str) -> Markup {
    html! { div class="provider-status" { span { (name) } strong { (state) } } }
}

fn sync_step(
    number: &str,
    icon: &str,
    title: &str,
    text: &str,
    state: &str,
    body: Markup,
) -> Markup {
    html! {
        article class="sync-step" data-sync-step=(number) data-sync-step-title=(title) data-step-state="waiting" {
            div class="sync-step-top" {
                span class="sync-step-number" { (number) }
                span class="sync-step-icon" aria-hidden="true" { (icon) }
                strong { (title) }
                b class="sync-step-badge" { (state) }
            }
            p { (text) }
            div class="sync-step-details" { (body) }
        }
    }
}

fn sync_detail(label: &str, value: &str) -> Markup {
    html! { span class="sync-detail" { em { (label) } strong { (value) } } }
}

fn storage_stat(label: &str, value: &str) -> Markup {
    html! { div class="storage-stat" { span { (label) } strong { (value) } } }
}

fn storage_legend(label: &str, value: &str, class: &str) -> Markup {
    html! { span class=(format!("storage-legend-item storage-legend-item--{}", class)) { em {} strong { (label) } small { (value) } } }
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

fn copy_button(label: &str, value: &str) -> Markup {
    html! { button class="btn btn--secondary" type="button" data-copy-value=(value) { (label) } }
}

fn system_service_row(
    name: &str,
    status: &str,
    last_changed: &str,
    restart: Option<(&str, &str)>,
) -> Markup {
    html! {
        div class="system-service-row" {
            span class="system-service-main" { strong { (name) } }
            b class=(format!("system-status system-status--{}", status.to_lowercase())) { (status) }
            small { "Last changed: " (last_changed) }
            span class="system-row-actions" {
                @if let Some((action, endpoint)) = restart {
                    (action_button(ButtonVariant::Secondary, "Restart", action, endpoint))
                }
            }
        }
    }
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

fn power_action(
    title: &str,
    text: &str,
    variant: ButtonVariant,
    action: &str,
    endpoint: &str,
) -> Markup {
    html! { article class="power-card" { h3 { (title) } p { (text) } (action_button(variant, title, action, endpoint)) } }
}

fn collapsible_log(title: &str, text: &str) -> Markup {
    html! { details class="collapsible-log" { summary { (title) } pre { code { (text) } } } }
}

fn nav_focus_button(label: &str, target: &str, focus: &str) -> Markup {
    html! { button class="btn btn--secondary" type="button" data-nav-target=(target) data-focus-target=(focus) { (label) } }
}

fn action_button(variant: ButtonVariant, label: &str, action: &str, endpoint: &str) -> Markup {
    html! { button class=(format!("btn btn--{}", variant.class())) type="button" data-button=(variant.class()) data-action=(action) data-endpoint=(endpoint) { (label) } }
}

fn link_button(variant: ButtonVariant, label: &str, action: &str, url: &str) -> Markup {
    html! { button class=(format!("btn btn--{}", variant.class())) type="button" data-button=(variant.class()) data-action=(action) data-url=(url) { (label) } }
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
                div class="product-mark" { "A" }
                h1 { "Arcadia Console" }
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
    PreEscaped(r#"(function(){document.documentElement.dataset.theme='dark';})();"#)
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
