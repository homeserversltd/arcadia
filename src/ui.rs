use maud::{html, Markup, PreEscaped, DOCTYPE};

use crate::{ButtonVariant, ConsoleStatus};

const FOLDERS: [&str; 12] = [
    "gba", "genesis", "snes", "nes", "ps1", "n64", "ps2", "sega-cd", "psp", "gamecube", "wii",
    "dos",
];

const VIEWS: [(&str, &str, &str); 9] = [
    ("home", "⌂", "Home"),
    ("games", "▣", "Games"),
    ("sync", "↻", "Sync"),
    ("ai-model", "◉", "AI Model"),
    ("lan-inference", "⇄", "LAN Inference"),
    ("access-pin", "●", "Access / PIN"),
    ("updates", "⬆", "Updates"),
    ("power", "⏻", "Power"),
    ("advanced", "⚙", "Advanced"),
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
                            (sync_view())
                            (ai_model_view())
                            (lan_inference_view())
                            (access_pin_view(status))
                            (updates_view(status))
                            (power_view())
                            (advanced_view(status))
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
                (status_badge("Network", "Online", "good", "The console is reachable on your home network."))
                (status_badge("GameScope", "Running", "good", "The TV game session is expected to be running."))
                (status_badge("Storage", "OK", "good", "Storage has room for games and artwork."))
                (status_badge("Sync", "Idle", "idle", "No game sync is running right now."))
                (status_badge("AI", "No Model", "idle", "No local model is currently marked as loaded."))
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
    view_shell("home", "Appliance overview", "Console Status", "This page manages your local game console. Copy games over the network, sync them into GameScope, and manage local AI.", html! {
        div class="status-card-grid" {
            (status_card("Network", "Online", "Your browser can reach the console management page."))
            (status_card("GameScope", "Running", "Games appear on the TV after sync creates shortcuts."))
            (status_card("Storage", "OK", "There is room for game files and artwork."))
            (status_card("Last Sync", "Not reported", "Run Sync Games after copying new files."))
            (status_card("Loaded AI Model", "No model loaded", "Load a model only when you need LAN inference."))
            (status_card("Software Version", status.arcadia.version, "Arcadia web console version."))
        }
        div class="primary-actions" {
            (action_button(ButtonVariant::Primary, "Sync Games", "sync-games", "/api/actions/sync-games"))
            (link_button(ButtonVariant::Secondary, "Open Games Folder", "open-games-folder", "smb://HOMECONSOLE"))
            (action_button(ButtonVariant::Secondary, "Check for Updates", "check-updates", "/api/actions/update-gui"))
        }
        (instruction_card("Where to go next", "Add Games shows the network folder names. Sync creates the GameScope entries. Advanced contains Linux details only when you need them."))
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

fn sync_view() -> Markup {
    view_shell("sync", "Library import", "Sync Games", "Sync scans the game folders, improves names and cover art when optional keys are present, then creates Steam shortcuts for GameScope.", html! {
        div class="task-hero" {
            div { strong { "Current state" } span id="sync-state" { "Idle" } }
            (action_button(ButtonVariant::Primary, "Start Sync", "sync-games", "/api/actions/sync-games"))
        }
        div class="status-card-grid" {
            (status_card("Last sync time", "Not reported", "The last completed scan time will appear here."))
            (status_card("Games found", "Not reported", "Files discovered in the console game folders."))
            (status_card("Steam shortcuts", "Not reported", "Entries created for GameScope / Steam mode."))
            (status_card("Artwork", "Ready", "Artwork improves when provider keys are configured."))
        }
        details class="settings-panel" {
            summary { "Configure Metadata Providers" }
            p { "Optional metadata keys improve cover art and game titles during Sync. Games still work without them." }
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
        (collapsible_log("Sync log preview", "No sync log has been loaded in this page. Start Sync and read the result message above."))
    })
}

fn ai_model_view() -> Markup {
    view_shell("ai-model", "Local inference", "Local AI Model", "The console can load one local model onto the GPU. Loading a model may take time and may reduce game performance while active.", html! {
        div class="active-model" {
            span { "Active model" }
            strong { "No model loaded" }
            p { "Only one model should be loaded at a time." }
        }
        div class="model-grid" {
            (model_card("Mistral 7B Instruct", "4.1 GB", "Q4_K_M", "6 GB VRAM", "Available", true))
            (model_card("Qwen2.5 Coder 7B", "4.7 GB", "Q4_K_M", "7 GB VRAM", "Available", false))
            (model_card("Llama 3.2 3B", "2.0 GB", "Q4_K_M", "4 GB VRAM", "Available", false))
        }
        div class="primary-actions" {
            (modal_button(ButtonVariant::Primary, "Load Selected Model", "Model loading", "This console will cold-load the selected local model onto the GPU. Games may run slower while a model is loaded."))
            (modal_button(ButtonVariant::Secondary, "Unload Model", "Unload model", "This removes the active local model from memory when the model server supports unloading."))
        }
        p class="warning" { "AI inference may affect game performance. Unload the model before playing demanding games." }
    })
}

fn lan_inference_view() -> Markup {
    view_shell("lan-inference", "Port 7777", "LAN Inference", "Devices on your home network can send prompts to the loaded local model through port 7777.", html! {
        (path_card("Local endpoint", "http://console.home.arpa:7777"))
        div class="status-card-grid" {
            (status_card("Port 7777", "Closed", "Enable LAN inference only on a trusted home network."))
            (status_card("Current model", "No model", "Load a local model before expecting useful replies."))
            (status_card("llama.cpp server", "Stopped", "The server opens the LAN endpoint when enabled."))
        }
        div class="primary-actions" {
            (modal_button(ButtonVariant::Primary, "Enable LAN Inference", "Enable LAN inference", "Only enable this on a trusted LAN. This is not intended for public internet exposure."))
            (modal_button(ButtonVariant::Secondary, "Disable LAN Inference", "Disable LAN inference", "This closes the local inference endpoint when the service supports it."))
        }
        p class="warning" { "Do not expose port 7777 to the public internet." }
        details class="collapsible-log" {
            summary { "Advanced curl example" }
            pre { code { "curl http://console.home.arpa:7777/v1/chat/completions -H 'Content-Type: application/json' -d '{\"messages\":[{\"role\":\"user\",\"content\":\"Hello\"}]}'" } }
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

fn advanced_view(status: &ConsoleStatus) -> Markup {
    view_shell("advanced", "Technical details", "Advanced", "This page exposes service details for technical users without polluting the normal console path.", html! {
        div class="status-card-grid" {
            (status_card("SSH", "Available when enabled", "Use SSH only if you administer the console."))
            (status_card("Hostname", status.surfaces.smb, "Network name for file shares and local access."))
            (status_card("IP address", "DHCP assigned", "Read the router or console TTY for the exact IP."))
            (status_card("Open ports", "80/8080, SMB, 7777 optional", "LAN inference stays local and should not be exposed to WAN."))
        }
        (instruction_card("TTY / Pi agent harness", "Advanced users may SSH or TTY into the console and talk to the Pi agent harness. Normal users do not need this path to add games, sync, update, or change PIN."))
        div class="service-list" {
            (service_status_row("gamescope", "Expected running"))
            (service_status_row("samba", "Shares game folders"))
            (service_status_row("sync service", "Runs on demand"))
            (service_status_row("llama.cpp server", "Optional"))
            (service_status_row("web GUI", "Arcadia"))
        }
        (collapsible_log("Advanced logs", "Logs are collapsed by default. Use system logs or Harmonia receipts for detailed diagnosis."))
    })
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

fn model_card(
    name: &str,
    size: &str,
    quant: &str,
    vram: &str,
    status: &str,
    selected: bool,
) -> Markup {
    html! {
        label class=(if selected { "model-card model-card--selected" } else { "model-card" }) {
            input type="radio" name="model" value=(name) checked[selected];
            span class="model-name" { (name) }
            span { "Size: " (size) }
            span { "Quantization: " (quant) }
            span { "VRAM estimate: " (vram) }
            strong { (status) }
        }
    }
}

fn service_status_row(service: &str, state: &str) -> Markup {
    html! { div class="service-status-row" { span { (service) } strong { (state) } } }
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
