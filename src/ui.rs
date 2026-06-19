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
    ("network", "⌁", "Network"),
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
                (nav_status_badge("Network", "Ethernet", "good", "Connected by Ethernet.", "network"))
                (nav_status_badge("GameScope", "Running", "good", "The TV game session is expected to be running.", "system"))
                (storage_status_badge(&status.storage.header_state, status.storage.header_class, &status.storage.header_tooltip))
                (status_badge("Sync", "Idle", "idle", "No game sync is running right now."))
                (status_badge("AI", "Not Loaded", "idle", "No local AI model is currently marked as loaded."))
                (status_badge("Update", "Current", "good", "No update is currently reported."))
                (status_badge("PIN", if status.gui_pin.pin_required { "Required" } else { "Open" }, if status.gui_pin.pin_required { "warn" } else { "idle" }, "Whether the web console asks for the setup PIN before opening."))
            }
        }
    }
}

fn status_badge(label: &str, state: &str, class: &str, help: &str) -> Markup {
    html! {
        div class=(format!("status-badge status-badge--{}", class)) title=(help) {
            span { (label) }
            strong { (state) }
        }
    }
}

fn nav_status_badge(label: &str, state: &str, class: &str, help: &str, target: &str) -> Markup {
    html! {
        button class=(format!("status-badge status-badge--{} status-badge--nav", class)) type="button" data-nav-target=(target) title=(help) aria-label=(format!("Open {}", label)) {
            span { (label) }
            strong { (state) }
        }
    }
}

fn storage_status_badge(state: &str, class: &str, help: &str) -> Markup {
    html! {
        button class=(format!("status-badge status-badge--{} status-badge--nav", class)) type="button" data-nav-target="storage" title=(help) aria-label="Open Storage" {
            span { "Storage" }
            strong { (state) }
        }
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

fn view_shell(id: &str, eyebrow: &str, title: &str, explanation: &str, body: Markup) -> Markup {
    html! {
        section id=(format!("view-{}", id)) class="view" data-view-panel=(id) tabindex="-1" {
            div class="view-heading" {
                p class="eyebrow" { (eyebrow) }
                h2 { (title) }
                p class="lede" { (explanation) }
            }
            (body)
        }
    }
}

fn home_view(status: &ConsoleStatus) -> Markup {
    view_shell("home", "HomeConsole launchpad", "Console Home", "Manage the local game console from here. Add games over the network, sync them into GameScope, or load a local AI model.", html! {
        (onboarding_card())

        section class="home-action-panel" aria-labelledby="home-primary-actions-title" {
            div class="section-heading" {
                h3 id="home-primary-actions-title" { "What do you want to do?" }
                p { "Choose the job first. Status is below when you need it." }
            }
            div class="home-action-grid" {
                (action_tile("▣", "Add Games", "Open the console’s network folders and copy games into the right system folder.", "games", "Network copy"))
                (action_tile("↻", "Sync Games", "Scan the game folders, fetch artwork, and add games to the GameScope library.", "sync", "Ready"))
                (action_tile("◉", "Local AI", "Choose the local AI that runs on this console and can be used on your home network.", "ai-model", "Not Loaded"))
            }
        }

        section class="home-section" aria-labelledby="home-status-title" {
            div class="section-heading section-heading--compact" {
                h3 id="home-status-title" { "Console Status" }
                p { "Health at a glance; actions stay above." }
            }
            div class="status-card-grid status-card-grid--compact" {
                (status_card("Network", "Online", "Console GUI reachable on the home network."))
                (status_card("GameScope", "Running", "Games appear on the TV after sync creates shortcuts."))
                (status_card("Storage", &format!("{} — {} free", status.storage.health, status.storage.free), &format!("{} used. Open Storage for the full breakdown.", status.storage.percent)))
                (status_card("Last Sync", "Not reported", "Run Sync Games after copying new files."))
                (status_card("Loaded Local AI", "Not Loaded", "Load local AI only when you need LAN inference."))
                (status_card("Software Version", status.arcadia.version, "Arcadia web console version."))
            }
        }

        section class="home-section home-recent" aria-labelledby="home-recent-title" {
            div class="section-heading section-heading--compact" {
                h3 id="home-recent-title" { "Recent Activity" }
                p { "Last sync is not reported yet. Local AI is not loaded." }
            }
            div class="inline-actions" {
                (nav_button("Open Games", "games"))
                (nav_button("Open Sync", "sync"))
                (nav_button("Open Local AI", "ai-model"))
            }
        }
    })
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

fn network_view(_status: &ConsoleStatus) -> Markup {
    view_shell("network", "Home network", "Network", "Manage how Arcadia connects to your home network. Network access is required for copying games, using the web console, and LAN inference.", html! {
        section class="network-summary-card" aria-labelledby="network-status-title" data-network-state="connected" {
            div class="section-heading" {
                h3 id="network-status-title" { "Connection Status" }
                p { "Arcadia is connected by Ethernet and can be reached from other devices on your LAN." }
            }
            div class="network-summary-main" {
                strong { "Connected by Ethernet" }
                span class="system-status system-status--running" { "Connected" }
            }
            div class="system-field-grid" {
                (system_field("Status", "Connected"))
                (system_field("Active connection", "Ethernet"))
                (system_field("Network name", "Wired LAN"))
                (system_field("IP address", "DHCP assigned"))
                (system_field("Signal strength", "Ethernet"))
                (system_field("Internet reachability", "Not Checked"))
            }
            p class="network-alt-copy" { "No network connection" }
            p class="network-alt-copy" { "Connect Ethernet or choose a Wi-Fi network to continue setup." }
        }

        section class="network-grid" aria-label="Network management" {
            article class="network-card" aria-labelledby="wifi-title" {
                div class="section-heading section-heading--compact" {
                    h3 id="wifi-title" { "Wi-Fi" }
                    p { "Choose a home Wi-Fi network when Ethernet is not connected." }
                }
                div class="system-field-grid" {
                    (system_field("Wi-Fi adapter status", "Enabled"))
                    (system_field("Current SSID", "Not connected"))
                    (system_field("Signal strength", "Not connected"))
                    (system_field("Security type", "WPA/WPA2 when available"))
                    (system_field("IP address", "Not connected"))
                    (system_field("Saved networks", "Show Saved Networks"))
                }
                div class="wifi-connect-panel" {
                    label { span { "SSID" } select class="field" name="wifi_ssid" { option { "Choose a network after scanning" } option { "HomeNetwork" } option { "Hidden network" } } }
                    label { span { "Hidden network name" } input class="field" type="text" name="hidden_ssid" autocomplete="off" placeholder="Hidden SSID"; }
                    label { span { "Wi-Fi password" } input class="field" type="password" name="wifi_password" autocomplete="new-password" placeholder="Password is never saved in the browser"; }
                    label class="wifi-show-password" { input type="checkbox" data-toggle-password="wifi_password"; span { "Show while typing" } }
                    div id="wifi-message" class="message" hidden {}
                }
                div class="inline-actions" {
                    (network_button("Scan for Networks", "scan-wifi"))
                    (network_button("Connect", "connect-wifi"))
                    (network_button("Disconnect", "disconnect-wifi"))
                    (network_button("Forget Network", "forget-wifi"))
                    (network_button("Show Saved Networks", "show-saved-wifi"))
                }
                p class="note" { "Could not connect to this Wi-Fi network. Check the password and try again." }
                p class="note" { "Wi-Fi signal is weak. Move Arcadia closer to the router or use Ethernet." }
            }

            article class="network-card" aria-labelledby="ethernet-title" {
                div class="section-heading section-heading--compact" {
                    h3 id="ethernet-title" { "Ethernet" }
                    p { "Ethernet is recommended for large game transfers and stable LAN inference." }
                }
                div class="system-field-grid" {
                    (system_field("Ethernet status", "Connected"))
                    (system_field("Link speed", "Not reported"))
                    (system_field("IP address", "DHCP assigned"))
                    (system_field("MAC address", "Not reported"))
                }
            }

            article class="network-card network-card--wide" aria-labelledby="device-addresses-title" {
                div class="section-heading section-heading--compact" {
                    h3 id="device-addresses-title" { "Device Addresses" }
                    p { "Use these addresses from devices on your home network." }
                }
                div class="system-endpoints" {
                    (command_box("Web console", "http://arcadia.home.arpa"))
                    (command_box("Games folder - Windows", "\\\\ARCADIA"))
                    (command_box("Games folder - Linux/macOS", "smb://ARCADIA"))
                    (command_box("LAN inference", "http://arcadia.home.arpa:7777"))
                }
            }

            article class="network-card network-card--wide" aria-labelledby="network-services-title" {
                div class="section-heading section-heading--compact" {
                    h3 id="network-services-title" { "Network Services" }
                    p { "Reachability for appliance services. Configuration stays on each service page." }
                }
                div class="network-service-list" {
                    (network_service_row("Web Console", "Available", "http://arcadia.home.arpa", None))
                    (network_service_row("Games Folder", "Available", "\\\\ARCADIA / smb://ARCADIA", Some("games")))
                    (network_service_row("LAN Inference", "Disabled", "7777", Some("lan-inference")))
                    (network_service_row("SSH", "Disabled", "22", Some("system")))
                }
                p class="warning" { "LAN Inference is intended only for trusted home networks. Do not expose port 7777 to the public internet." }
            }
        }
    })
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
        "View technical console status, service health, networking details, SSH access, and logs.",
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
                        (system_service_row("Web GUI", "Runs this management interface.", "Running", "Now", None))
                    }
                }

                article class="system-card system-card--logs" aria-labelledby="system-logs-title" {
                    div class="section-heading section-heading--compact" {
                        h3 id="system-logs-title" { "Logs" }
                        p { "Logs help diagnose problems. They are mostly useful for support or technical users." }
                    }
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

fn onboarding_card() -> Markup {
    html! {
        section class="onboarding-card" data-onboarding-card="true" aria-labelledby="onboarding-title" {
            div class="onboarding-head" {
                div {
                    p class="eyebrow" { "First setup" }
                    h3 id="onboarding-title" { "Welcome to Arcadia" }
                    p { "Finish these steps to set up your local game console." }
                }
                button class="btn btn--secondary" type="button" data-onboarding-hide-session="true" { "Hide for now" }
            }
            ol class="onboarding-checklist" {
                (onboarding_step("1", "Connect to Network", "Connect Arcadia to your home network so other devices can copy games to it and use Local AI.", "Ready", "Open Network Settings", "network", "Connected by Ethernet", "Connected to Wi-Fi", "No network connection"))
                (onboarding_step("2", "Add Games", "Copy games into Arcadia’s network folders from another device on your home network.", "Ready", "Add Games", "games", "", "", ""))
                (onboarding_step("3", "Run First Sync", "Sync turns copied files into playable GameScope entries with artwork and titles when available.", "Not Started", "Start Sync", "sync", "", "", ""))
                (onboarding_step("4", "Start Playing", "Open the GameScope library and launch your games from the console interface.", "Not Started", "Return to Console", "power", "GameScope is not running", "Restart GameScope", ""))
            }
        }
    }
}

fn onboarding_step(
    number: &str,
    title: &str,
    text: &str,
    state: &str,
    action: &str,
    target: &str,
    note_a: &str,
    note_b: &str,
    note_c: &str,
) -> Markup {
    html! {
        li class="onboarding-step" data-onboarding-step=(number) data-onboarding-state=(state) {
            span class="onboarding-number" { (number) }
            div class="onboarding-copy" {
                strong { (title) }
                p { (text) }
                @if !note_a.is_empty() { small { (note_a) } }
                @if !note_b.is_empty() { small { (note_b) } }
                @if !note_c.is_empty() { small { (note_c) } }
            }
            b class="onboarding-state" { (state) }
            (nav_button(action, target))
        }
    }
}

fn network_button(label: &str, action: &str) -> Markup {
    html! { button class="btn btn--secondary" type="button" data-network-action=(action) { (label) } }
}

fn network_service_row(name: &str, status: &str, address: &str, target: Option<&str>) -> Markup {
    html! {
        div class="network-service-row" {
            span { strong { (name) } em { (address) } }
            b class=(format!("system-status system-status--{}", status.to_lowercase())) { (status) }
            @if let Some(view) = target { (nav_button("Open", view)) } @else { span class="network-service-spacer" {} }
        }
    }
}

fn status_card(title: &str, value: &str, help: &str) -> Markup {
    html! { article class="status-card" { span { (title) } strong { (value) } p { (help) } } }
}

fn action_tile(icon: &str, title: &str, text: &str, view: &str, badge: &str) -> Markup {
    html! {
        button class="home-action-tile" type="button" data-nav-target=(view) aria-label=(format!("{}: {}", title, text)) {
            span class="home-action-icon" aria-hidden="true" { (icon) }
            span class="home-action-copy" {
                strong { (title) }
                span { (text) }
            }
            span class="home-action-badge" { (badge) }
        }
    }
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
                (modal_button(ButtonVariant::Secondary, "View Logs", &format!("{} logs", name), "Open the Logs section below for redacted troubleshooting output."))
            }
        }
    }
}

fn system_log_group(name: &str, text: &str) -> Markup {
    html! {
        details class="collapsible-log system-log-group" {
            summary { (name) }
            p { (text) }
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
