use maud::{html, Markup, PreEscaped, DOCTYPE};

use crate::{ButtonVariant, ConsoleStatus};

const FOLDERS: [&str; 12] = [
    "gba", "genesis", "snes", "nes", "ps1", "n64", "ps2", "sega-cd", "psp", "gamecube", "wii",
    "dos",
];

const VIEWS: [(&str, &str, &str); 11] = [
    ("home", "⌂", "Home"),
    ("games", "▣", "Games"),
    ("sync", "↻", "Sync"),
    ("storage", "▰", "Storage"),
    ("ai-model", "◉", "Local AI"),
    ("lan-inference", "⇄", "LAN Inference"),
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
                            (lan_inference_view())
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
    html! {
        header class="top-header" {
            div class="product-lockup" {
                div class="product-mark" { "A" }
                div {
                    h1 { "Arcadia Console" }
                    p { (status.canonical_url.trim_end_matches('/')) }
                }
            }
            div class="header-indicators" aria-label="Console state" {
                (nav_status_badge("network", if status.network.online { "◌" } else { "!" }, if status.network.online { &status.network.connection_type } else { "Offline" }, if status.network.online { "good" } else { "bad" }, "Open Network", "network"))
                (nav_status_badge("gamescope", "▶", title_case_state(status.arcadia.service), if status.arcadia.service == "running" { "good" } else { "idle" }, "Open System", "system"))
                (nav_status_badge("storage", "▰", &status.storage.free, status.storage.header_class, &status.storage.header_tooltip, "storage"))
                (status_badge("sync", "↻", if status.library.sync_needed { "Needed" } else { "Idle" }, if status.library.sync_needed { "warn" } else { "idle" }, "Game sync state."))
                (status_badge("ai", "◉", if status.local_ai.loaded_model.is_some() { "Loaded" } else { "No AI" }, if status.local_ai.loaded_model.is_some() { "good" } else { "idle" }, "Local AI state."))
                (status_badge("pin", "●", if status.gui_pin.pin_required { "PIN" } else { "Open" }, if status.gui_pin.pin_required { "warn" } else { "idle" }, "GUI PIN state."))
            }
        }
    }
}

fn status_badge(kind: &str, icon: &str, state: &str, class: &str, help: &str) -> Markup {
    html! {
        div class=(format!("status-badge status-badge--{}", class)) title=(help) data-chip-kind=(kind) aria-label=(format!("{}: {}", kind, state)) {
            span class="chip-icon" aria-hidden="true" { (icon) }
            strong { (state) }
        }
    }
}

fn nav_status_badge(
    kind: &str,
    icon: &str,
    state: &str,
    class: &str,
    help: &str,
    target: &str,
) -> Markup {
    html! {
        button class=(format!("status-badge status-badge--{} status-badge--nav", class)) type="button" data-nav-target=(target) data-chip-kind=(kind) title=(help) aria-label=(format!("{}: {}", kind, state)) {
            span class="chip-icon" aria-hidden="true" { (icon) }
            strong { (state) }
        }
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
                        span { "Open LAN settings or unload the failed model." }
                        (nav_button("LAN Settings", "lan-inference"))
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
    } else if status.library.sync_needed {
        (
            format!(
                "{} games waiting for first sync",
                status.library.detected_files
            ),
            format!("{} GameScope entries", status.library.gamescope_entries),
            Some("Start Sync"),
            None,
            Some("/api/actions/sync-games"),
        )
    } else if status.library.last_sync_state == "error" {
        (
            "Sync failed".to_string(),
            "Open Sync for the latest receipt.".to_string(),
            Some("View Sync"),
            Some("sync"),
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
            Some("LAN Settings"),
            Some("lan-inference"),
            None,
        )
    } else {
        (
            "Ready".to_string(),
            format!(
                "GameScope {}",
                title_case_state(status.arcadia.service).to_lowercase()
            ),
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
                (storage_mini_row("Games", &status.storage.games.size, status.storage.games.percent_of_total))
                (storage_mini_row("Artwork", &status.storage.artwork.size, status.storage.artwork.percent_of_total))
                (storage_mini_row("AI Models", &status.storage.ai_models.size, status.storage.ai_models.percent_of_total))
                (storage_mini_row("Other", &status.storage.other.size, status.storage.other.percent_of_total))
            }
            div class="inline-actions inline-actions--compact" {
                (nav_button("Open Storage", "storage"))
                @if status.storage.artwork.bytes >= 1_000_000_000 { (action_button(ButtonVariant::Secondary, "Clean Artwork", "clear-artwork-cache", "/api/actions/clear-artwork-cache")) }
                @if status.storage.ai_models.bytes > 0 { (nav_button("Manage Models", "ai-model")) }
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
                    p class="card-line" { (status.network.ip_address) " · " (status.network.ethernet_speed_mbps.map(|v| format!("{} Mbps", v)).unwrap_or_else(|| "Unknown speed".to_string())) }
                }
                div class="reachability-row reachability-row--topology" {
                    (reachability("Console", status.network.console_reachable))
                    (reachability("Folders", status.network.samba_reachable))
                    (reachability("LAN AI", status.network.lan_ai_reachable))
                    @if let Some(internet) = status.network.internet_reachable { (reachability("Internet", internet)) }
                }
                div class="inline-actions inline-actions--compact" {
                    (nav_button("Manage Wi-Fi", "network"))
                    (copy_button("Copy folders", &format!("smb://{}", status.surfaces.smb)))
                    (copy_button("Copy console URL", status.canonical_url.trim_end_matches('/')))
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
            div class="card-head" { h3 { "Game Library" } strong { (status.library.detected_files) " files" } }
            div class="state-rows state-rows--compact" {
                (state_row("Detected files", &status.library.detected_files.to_string()))
                (state_row("GameScope entries", &status.library.gamescope_entries.to_string()))
                (state_row("Last sync", &status.library.last_sync))
                (state_row("Artwork", &status.library.artwork_status))
            }
            div class="inline-actions inline-actions--compact" {
                @if status.library.sync_needed || status.library.last_sync_state == "error" { (action_button(ButtonVariant::Primary, "Start Sync", "sync-games", "/api/actions/sync-games")) }
                @else { (nav_button("View Sync", "sync")) }
            }
        }
    }
}

fn home_local_ai_card(status: &ConsoleStatus) -> Markup {
    let load_state = title_case_state_like(&status.local_ai.load_state);
    let selected = status
        .local_ai
        .selected_model_name
        .as_deref()
        .unwrap_or("No model selected");
    let gpu = status.local_ai.gpu_memory.as_deref().unwrap_or("Unknown");
    let lan = if let Some(port) = status.local_ai.lan_inference_port {
        format!("On · :{}", port)
    } else {
        "Off".to_string()
    };
    html! {
        article class=(if status.local_ai.load_state == "error" { "operational-card local-ai-home-card attention" } else { "operational-card local-ai-home-card" }) {
            div class="card-head" { h3 { "Local AI" } strong { (load_state) } }
            label class="compact-select-label" { span { "Model" } select class="compact-select" name="home-local-ai-model" {
                @if status.local_ai.available_models.is_empty() {
                    option value="" { "No local models found" }
                } @else {
                    @for model in &status.local_ai.available_models {
                        option value=(model.id) selected[status.local_ai.selected_model_id.as_deref() == Some(model.id.as_str())] { (model.name) }
                    }
                }
            } }
            div class="state-rows state-rows--compact" {
                (state_row("Selected", selected))
                (state_row("GPU", gpu))
                (state_row("LAN", &lan))
            }
            @if let (Some(used), Some(total)) = (status.local_ai.gpu_memory_used_bytes, status.local_ai.gpu_memory_total_bytes) {
                div class="gpu-bar" aria-label="GPU memory usage" { span style=(format!("width: {}%", ((used.saturating_mul(100) / total.max(1)).min(100)))) {} }
            }
            div class="inline-actions inline-actions--compact" {
                @if status.local_ai.load_state == "hot" { (modal_button(ButtonVariant::Secondary, "Unload", "Unload local AI", "Unload the active local AI model when the backend control is connected.")) }
                @else { (modal_button(ButtonVariant::Primary, "Load", "Load local AI", "Load the selected local AI model when the backend control is connected.")) }
                (nav_button("LAN Settings", "lan-inference"))
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

fn storage_mini_row(label: &str, value: &str, percent: u8) -> Markup {
    html! { div class="storage-mini-row" { span { (label) } strong { (value) } em { (percent) "%" } } }
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
                (path_card("Linux / macOS", "smb://HOMECONSOLE"))
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
                (link_button(ButtonVariant::Primary, "Open Games Folder", "open-games-folder", &format!("smb://{}", status.surfaces.smb)))
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
                (link_button(ButtonVariant::Secondary, "Open Games Folder", "open-games-folder", &format!("smb://{}", status.surfaces.smb)))
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
            (link_button(ButtonVariant::Secondary, "Open Games Folder", "open-games-folder", &format!("smb://{}", status.surfaces.smb)))
            a class="btn btn--secondary" href="#sync-provider-settings" { "Configure Metadata Providers" }
            a class="btn btn--secondary" href="#sync-log-panel" { "View Sync Log" }
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
        details id="sync-log-panel" class="collapsible-log" {
            summary { "View Sync Log" }
            pre { code id="sync-log-output" { "Logs are secondary. Start Sync to see the latest result message here." } }
        }
        div id="console-action-message" class="message" hidden {}
    })
}

fn storage_view(status: &ConsoleStatus) -> Markup {
    view_shell("storage", "Local disk", "Storage", "See what is using space on the console. Games, artwork, and local AI models are stored on the device and can grow over time.", html! {
        section class="storage-summary" aria-labelledby="free-space-title" {
            div class="section-heading" {
                h3 id="free-space-title" { "Free Space" }
                p { (status.storage.warning_copy) }
            }
            div class="storage-hero" {
                div class="storage-hero-main" {
                    span { "Storage " (status.storage.health) }
                    strong { (status.storage.free) " free" }
                    p { (status.storage.percent) " used" }
                }
                div class="storage-hero-stats" {
                    (storage_stat("Total storage", &status.storage.total))
                    (storage_stat("Used storage", &status.storage.used))
                    (storage_stat("Free storage", &status.storage.free))
                    (storage_stat("Percent used", &status.storage.percent))
                }
            }
            div class="storage-bar" aria-label="Segmented storage usage" {
                span class="storage-segment storage-segment--games" style=(format!("width: {}%", status.storage.games.percent_of_total)) title=(format!("Games {}", status.storage.games.size)) {}
                span class="storage-segment storage-segment--artwork" style=(format!("width: {}%", status.storage.artwork.percent_of_total)) title=(format!("Artwork {}", status.storage.artwork.size)) {}
                span class="storage-segment storage-segment--ai" style=(format!("width: {}%", status.storage.ai_models.percent_of_total)) title=(format!("AI Models {}", status.storage.ai_models.size)) {}
                span class="storage-segment storage-segment--other" style=(format!("width: {}%", status.storage.other.percent_of_total)) title=(format!("Other {}", status.storage.other.size)) {}
                span class="storage-segment storage-segment--free" style=(format!("width: {}%", 100u8.saturating_sub(status.storage.percent_used))) title=(format!("Free {}", status.storage.free)) {}
            }
            div class="storage-legend" {
                (storage_legend("Games", &status.storage.games.size, "games"))
                (storage_legend("Artwork", &status.storage.artwork.size, "artwork"))
                (storage_legend("AI Models", &status.storage.ai_models.size, "ai"))
                (storage_legend("Other", &status.storage.other.size, "other"))
                (storage_legend("Free Space", &status.storage.free, "free"))
            }
        }

        section class="storage-category-grid" aria-label="Storage categories" {
            (storage_category("Games", &status.storage.games.size, &status.storage.games.meta, &status.storage.games.detail, html! {
                (link_button(ButtonVariant::Primary, "Open Games Folder", "open-games-folder", &format!("smb://{}", status.surfaces.smb)))
                (modal_button(ButtonVariant::Secondary, "Rescan Game Storage", "Rescan game storage", "Refresh this page to rescan game storage from the console."))
            }))
            (storage_category("Artwork", &status.storage.artwork.size, &status.storage.artwork.meta, "Last artwork sync: Not reported. Clearing artwork does not delete games. Artwork can be downloaded again during Sync.", html! {
                (action_button(ButtonVariant::Danger, "Clear Artwork Cache", "clear-artwork-cache", "/api/actions/clear-artwork-cache"))
                (modal_button(ButtonVariant::Secondary, "Rebuild Artwork on Next Sync", "Rebuild artwork", "The next Sync will rebuild artwork and metadata for copied games."))
            }))
            (storage_category("AI Models", &status.storage.ai_models.size, &status.storage.ai_models.meta, &status.storage.ai_models.detail, html! {
                @if status.storage.ai_models.models.is_empty() {
                    p { "No local AI model files are installed." }
                } @else {
                    @for model in &status.storage.ai_models.models {
                        (ai_model_storage_row(&model.friendly_name, &model.filename, &model.size, model.status))
                    }
                }
                (nav_button("Open Local AI", "ai-model"))
            }))
            (storage_category("Other Storage", &status.storage.other.size, &status.storage.other.meta, &status.storage.other.detail, html! {
                (action_button(ButtonVariant::Secondary, "Clean Temporary Files", "clean-temporary-files", "/api/actions/clean-temporary-files"))
            }))
        }
    })
}

fn ai_model_view(status: &ConsoleStatus) -> Markup {
    view_shell("ai-model", "On-device assistant", "Local AI", "This console can run a local AI assistant without sending prompts to the cloud. Choose which model is loaded and whether it is available to devices on your home network.", html! {
        section class="local-ai-section" aria-labelledby="loaded-model-title" {
            div class="section-heading section-heading--compact" {
                h3 id="loaded-model-title" { "Loaded Model" }
                p { "No local AI model is currently loaded." }
            }
            div class="active-model" {
                span { "Status" }
                strong { "Not Loaded" }
                p { "Only one model should be loaded at a time. Loading a model may take several seconds or minutes depending on size." }
                div class="model-meta-row" {
                    (model_meta("Current model", "None"))
                    (model_meta("Memory estimate", "0 GB GPU memory"))
                    (model_meta("LAN inference", "Off"))
                }
                div class="inline-actions" {
                    (modal_button(ButtonVariant::Secondary, "Unload", "Unload local AI", "This removes the active local AI model from memory when unloading is supported."))
                    (modal_button(ButtonVariant::Secondary, "Reload", "Reload local AI", "Reload the selected local AI model if it is already configured."))
                    (nav_button("Open LAN Inference Settings", "lan-inference"))
                }
            }
        }

        section class="local-ai-section" aria-labelledby="available-models-title" {
            div class="section-heading section-heading--compact" {
                h3 id="available-models-title" { "Available Models" }
                p { "Choose the local AI that should run on this console. Model details are secondary." }
            }
            div class="model-grid" {
                (model_card("Mistral 7B Instruct", "mistral-7b-instruct.Q4_K_M.gguf", "4.1 GB", "Q4_K_M", "6 GB", "Balanced", "Available", true))
                (model_card("Qwen2.5 Coder 7B", "qwen2.5-coder-7b.Q4_K_M.gguf", "4.7 GB", "Q4_K_M", "7 GB", "Higher Quality", "Available", false))
                (model_card("Llama 3.2 3B", "llama-3.2-3b.Q4_K_M.gguf", "2.0 GB", "Q4_K_M", "4 GB", "Fast", "Available", false))
            }
        }

        section class="local-ai-section" aria-labelledby="gpu-usage-title" {
            @if status.storage.percent_used >= 90 { p class="warning" { "Storage is low. Remove unused games, artwork, or AI models before adding more models." } }
            div class="section-heading section-heading--compact" {
                h3 id="gpu-usage-title" { "GPU Usage" }
                p { "Local AI uses the same GPU as games. Large models may reduce game performance while loaded." }
            }
            div class="status-card-grid status-card-grid--compact" {
                (status_card("GPU memory used", "0 GB", "No local AI model is loaded."))
                (status_card("GPU memory available", "Not reported", "Available GPU memory appears here when telemetry is connected."))
                (status_card("AI process", "Stopped", "Local AI is not running right now."))
            }
        }
    })
}

fn lan_inference_view() -> Markup {
    view_shell("lan-inference", "Port 7777", "LAN Inference", "Devices on your home network can send prompts to the loaded local model through port 7777.", html! {
        (path_card("Local endpoint", "http://console.home.arpa:7777"))
        div class="status-card-grid" {
            (status_card("Port 7777", "Closed", "Enable LAN inference only on a trusted home network."))
            (status_card("Current model", "No model", "Load a local model before expecting useful replies."))
            (status_card("Local AI server", "Stopped", "The server opens the LAN endpoint when enabled."))
        }
        div class="primary-actions" {
            (modal_button(ButtonVariant::Primary, "Enable LAN Inference", "Enable LAN inference", "Only enable this on a trusted LAN. This is not intended for public internet exposure."))
            (modal_button(ButtonVariant::Secondary, "Disable LAN Inference", "Disable LAN inference", "This closes the local inference endpoint when the service supports it."))
        }
        p class="warning" { "Do not expose port 7777 to the public internet." }
        details class="collapsible-log" {
            summary { "Curl example" }
            pre { code { "curl http://console.home.arpa:7777/v1/chat/completions -H 'Content-Type: application/json' -d '{\"messages\":[{\"role\":\"user\",\"content\":\"Hello\"}]}'" } }
        }
    })
}

fn network_view(status: &ConsoleStatus) -> Markup {
    view_shell(
        "network",
        "Connection",
        "Network",
        "Manage Wi-Fi and copy addresses.",
        html! {
            article class="system-card system-card--networking" {
                h3 { "Connection" }
                div class="system-field-grid" {
                    (system_field("Type", &status.network.connection_type))
                    (system_field("Address", &status.network.ip_address))
                    (system_field("Signal", status.network.signal.as_deref().unwrap_or("—")))
                }
                div class="system-endpoints" {
                    (command_box("Console", status.canonical_url.trim_end_matches('/')))
                    (command_box("Game folders", &format!("smb://{}", status.surfaces.smb)))
                    (command_box("mDNS", status.surfaces.mdns))
                }
            }
        },
    )
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
            section class="system-grid" aria-label="System support panel" {
                article class="system-card system-card--ssh" aria-labelledby="system-ssh-title" {
                    div class="section-heading section-heading--compact" {
                        h3 id="system-ssh-title" { "SSH" }
                        p { "SSH is for direct technical access to the console. Normal game management does not require SSH." }
                    }
                    div class="system-field-grid" {
                        (system_field("SSH status", "Disabled"))
                        (system_field("Hostname", "console.home.arpa"))
                        (system_field("LAN IP address", "DHCP assigned"))
                        (system_field("Username", "console"))
                    }
                    (command_box("Example command", "ssh console@console.home.arpa"))
                    p class="warning" { "Only enable SSH on a trusted home network. Use a strong password or key-based access." }
                    div class="inline-actions" {
                        (modal_button(ButtonVariant::Secondary, "Enable SSH", "Enable SSH", "Enable SSH only on a trusted home network. Use a strong password or key-based access."))
                        (modal_button(ButtonVariant::Secondary, "Disable SSH", "Disable SSH", "Disable direct shell access when it is not needed."))
                        (copy_button("Copy SSH Command", "ssh console@console.home.arpa"))
                    }
                }

                article class="system-card system-card--services" aria-labelledby="system-services-title" {
                    div class="section-heading section-heading--compact" {
                        h3 id="system-services-title" { "Services" }
                        p { "Health for the console services behind the normal pages." }
                    }
                    div class="system-service-list" {
                        (system_service_row("GameScope", "Runs the console gaming session.", "Running", "Not reported", Some(("restart-gamescope", "/api/actions/restart-gamescope"))))
                        (system_service_row("Samba", "Shares game folders over the home network.", "Running", "Not reported", None))
                        (system_service_row("Game Sync", "Adds copied games to the GameScope library.", "Stopped", "Runs on demand", None))
                        (system_service_row("Local AI", "Loads the selected local AI model.", "Stopped", "Not reported", None))
                        (system_service_row("LAN Inference", "Lets other home-network devices use Local AI.", "Stopped", "Not reported", None))
                        (system_service_row("Web GUI", "Runs this management interface.", "Running", "Current", None))
                    }
                }

                article class="system-card system-card--logs" aria-labelledby="system-logs-title" {
                    span id="system-logs-title" class="sr-only" { "Logs" }
                    (system_log_group("Sync Log", "Sync log output is redacted before display. Provider API keys and saved PINs are never shown."))
                    (system_log_group("Local AI Log", "Local AI service messages appear here when connected to the log reader."))
                    (system_log_group("LAN Inference Log", "LAN inference service messages appear here when connected to the log reader."))
                    (system_log_group("System Log", "System service events appear here when connected to the log reader."))
                    (system_log_group("Web GUI Log", "Arcadia web GUI messages appear here when connected to the log reader."))
                }

                article class="system-card system-card--networking" aria-labelledby="system-networking-title" {
                    div class="section-heading section-heading--compact" {
                        h3 id="system-networking-title" { "Networking" }
                        p { "How the console is reached on the home network." }
                    }
                    div class="system-field-grid" {
                        (system_field("Hostname", "console.home.arpa"))
                        (system_field("Local domain/path", status.canonical_url.trim_end_matches('/')))
                        (system_field("LAN IP address", "DHCP assigned"))
                        (system_field("MAC address", "Not reported"))
                        (system_field("Network status", "Online"))
                        (system_field("Active interface", "LAN"))
                    }
                    div class="system-endpoints" {
                        (command_box("Web GUI", "http://console.home.arpa"))
                        (command_box("Games Folder", "\\\\HOMECONSOLE"))
                        (command_box("LAN Inference", "http://console.home.arpa:7777"))
                    }
                    h4 { "Open local ports" }
                    div class="system-port-list" {
                        (system_field("80/443", "Web GUI"))
                        (system_field("445", "Samba"))
                        (system_field("7777", "LAN Inference, only when enabled"))
                        (system_field("22", "SSH, only when enabled"))
                    }
                    p class="warning" { "LAN Inference is intended only for trusted home networks. Do not expose port 7777 to the public internet." }
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

fn storage_category(title: &str, value: &str, meta: &str, text: &str, actions: Markup) -> Markup {
    html! {
        article class="storage-category-card" {
            h3 { (title) }
            strong class="storage-category-value" { (value) }
            span class="storage-category-meta" { (meta) }
            p { (text) }
            div class="inline-actions" { (actions) }
        }
    }
}

fn ai_model_storage_row(name: &str, filename: &str, size: &str, status: &str) -> Markup {
    html! {
        div class="storage-model-row" {
            span { strong { (name) } code { (filename) } }
            em { (size) }
            b { (status) }
            (action_button(ButtonVariant::Danger, "Remove Model", "remove-ai-model", &format!("/api/actions/remove-ai-model?name={}", filename)))
        }
    }
}

fn model_card(
    name: &str,
    filename: &str,
    size: &str,
    quant: &str,
    gpu_memory: &str,
    recommended_use: &str,
    status: &str,
    selected: bool,
) -> Markup {
    html! {
        article class=(if selected { "model-card model-card--selected" } else { "model-card" }) {
            label class="model-choice" {
                input type="radio" name="model" value=(name) checked[selected];
                span class="model-name" { (name) }
            }
            code class="model-filename" { (filename) }
            div class="model-meta-row" {
                (model_meta("Size", size))
                (model_meta("Quantization", quant))
                (model_meta("GPU memory", gpu_memory))
                (model_meta("Recommended use", recommended_use))
            }
            strong class="model-status" { (status) }
            (modal_button(ButtonVariant::Primary, "Load This Model", "Load local AI", "Load this local AI model onto the console GPU. Games may run slower while a model is loaded."))
        }
    }
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
    description: &str,
    status: &str,
    last_changed: &str,
    restart: Option<(&str, &str)>,
) -> Markup {
    html! {
        div class="system-service-row" {
            span class="system-service-main" { strong { (name) } em { (description) } }
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

fn system_log_group(name: &str, text: &str) -> Markup {
    html! {
        details class="collapsible-log system-log-group" {
            summary { (name) }
            div class="inline-actions" {
                (modal_button(ButtonVariant::Secondary, "View", name, text))
                (copy_button("Copy", text))
                (modal_button(ButtonVariant::Secondary, "Download", name, "Downloadable redacted logs will be available when the log bundle endpoint is connected."))
            }
            pre { code { (text) } }
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
