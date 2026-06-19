use maud::{html, Markup, PreEscaped, DOCTYPE};

use crate::{ButtonVariant, ConsoleStatus};

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
                div id="app" class="console-shell" aria-hidden=(status.gui_pin.pin_required) {
                    main class="console-pane" aria-label="HomeConsole" {
                        section class="control-grid" {
                            article class="panel" data-module="status" {
                                div class="status-strip" {
                                    (status_card("Network", "Online"))
                                    (status_card("Runtime", &status.runtime.machine_uptime))
                                    (status_card("GUI PIN", if status.gui_pin.pin_required { "required" } else { "off" }))
                                }
                            }

                            article class="panel panel--controls" data-module="actions" {
                                div class="button-row button-row--five" {
                                    (action_button(ButtonVariant::Primary, "Update GUI", "update-gui", "/api/actions/update-gui"))
                                    (action_button(ButtonVariant::Primary, "Sync games", "sync-games", "/api/actions/sync-games"))
                                    (link_button(ButtonVariant::Secondary, "Add games", "add-games", "smb://HOMECONSOLE"))
                                    (action_button(ButtonVariant::Secondary, "Reboot console", "reboot-console", "/api/actions/reboot-console"))
                                    (action_button(ButtonVariant::Danger, "Shut down console", "shutdown-console", "/api/actions/shutdown-console"))
                                }
                                div id="console-action-message" class="message" hidden {}
                            }

                            article class="panel" data-module="smb-folders" {
                                (smb_explainer(status))
                            }

                            article class="panel" data-module="provider-keys" {
                                p class="tile-note" { "Provider keys let Sync games fetch artwork and metadata. Values are saved for the console and are not shown back." }
                                form id="provider-keys-form" class="stack" autocomplete="off" {
                                    label { span { "SteamGridDB API key" } input class="field" type="password" name="steamgriddb_api_key" autocomplete="off"; }
                                    label { span { "TheGamesDB API key" } input class="field" type="password" name="thegamesdb_api_key" autocomplete="off"; }
                                    label { span { "ScreenScraper API key" } input class="field" type="password" name="screenscraper_api_key" autocomplete="off"; }
                                    div id="provider-keys-message" class="message" hidden {}
                                    button class="btn btn--primary" type="submit" { "Save API keys" }
                                }
                            }

                            article class="panel" data-module="gui-pin" {
                                div class="module-grid module-grid--forms" {
                                    article class="submodule control-card" {
                                        p { "GUI PIN controls whether Arcadia asks for a PIN before opening. The PIN is stored by Keyman; Arcadia never shows it back." }
                                        div class="button-row" data-module="gui-pin-access" {
                                            button class="btn btn--secondary" type="button" data-action="gui-pin-enable" data-endpoint="/api/gui-pin/access" data-pin-required="true" { "Require GUI PIN" }
                                            button class="btn btn--secondary" type="button" data-action="gui-pin-disable" data-endpoint="/api/gui-pin/access" data-pin-required="false" { "Open without PIN" }
                                        }
                                        div id="gui-pin-access-message" class="message" hidden {}
                                    }
                                    article class="submodule control-card" {
                                        p { "Change the GUI PIN when the current PIN is known." }
                                        form id="gui-pin-change-form" class="stack" autocomplete="off" {
                                            label { span { "Current PIN" } input class="field" type="password" name="current_pin" autocomplete="current-password" required; }
                                            label { span { "New PIN" } input class="field" type="password" name="new_pin" autocomplete="new-password" required minlength="4"; }
                                            label { span { "Confirm new PIN" } input class="field" type="password" name="confirm_pin" autocomplete="new-password" required minlength="4"; }
                                            div id="gui-pin-change-message" class="message" hidden {}
                                            button class="btn btn--primary" type="submit" { "Change GUI PIN" }
                                        }
                                    }
                                }
                            }

                        }
                    }
                }
                (modal_root())
                script src="/static/app.js" {}
            }
        }
    }
}

fn gui_pin_gate(status: &ConsoleStatus) -> Markup {
    html! {
        section id="gui-pin-gate" class="pin-auth-container" data-required=(status.gui_pin.pin_required) {
            article class="panel pin-auth-card" {
                h1 { "HomeConsole" }
                h2 { "GUI PIN" }
                p class="tile-note" { "Enter the GUI PIN to open Arcadia." }
                form id="gui-pin-unlock-form" class="stack" autocomplete="off" {
                    input class="field field--pin" type="password" name="pin" placeholder="Enter GUI PIN" autocomplete="current-password" autofocus;
                    div id="gui-pin-auth-error" class="message message--error" hidden {}
                    button class="btn btn--primary" type="submit" { "Open Arcadia" }
                }
                small class="mono" { "PIN storage: " (status.gui_pin.pin_storage) }
            }
        }
    }
}

fn status_card(title: &str, value: &str) -> Markup {
    html! {
        div class="status-card" {
            span { (title) }
            strong { (value) }
        }
    }
}

fn smb_explainer(status: &ConsoleStatus) -> Markup {
    html! {
        div class="text-block" {
            p {
                "Copy games and media at "
                code { "\\\\" (status.surfaces.smb) }
                " or "
                code { "smb://" (status.surfaces.smb) }
                "."
            }
            ol class="numbered-list" {
                li { strong { "Folders" } span { "Use the matching folder: gba, genesis, snes, nes, ps1, n64, ps2, sega-cd, psp, gamecube, wii, or dos." } }
                li { strong { "Upload" } span { "Copy files into the matching folder and wait for the copy to finish." } }
                li { strong { "Sync" } span { "Sync games scans those folders, creates Steam shortcuts, and adds artwork when keys are present." } }
            }
            p class="tile-note" {
                "SMB writes to console storage on the local network."
            }
        }
    }
}

fn theme_boot_script() -> PreEscaped<&'static str> {
    PreEscaped(
        r#"(function(){try{var t=localStorage.getItem('arcadia-theme');if(t!=='light'&&t!=='dark'){t=(window.matchMedia&&window.matchMedia('(prefers-color-scheme: dark)').matches)?'dark':'light';}document.documentElement.dataset.theme=t;}catch(_){document.documentElement.dataset.theme='light';}})();"#,
    )
}

fn action_button(variant: ButtonVariant, label: &str, action: &str, endpoint: &str) -> Markup {
    html! {
        button class=(format!("btn btn--{}", variant.class())) type="button" data-button=(variant.class()) data-action=(action) data-endpoint=(endpoint) { (label) }
    }
}

fn link_button(variant: ButtonVariant, label: &str, action: &str, url: &str) -> Markup {
    html! {
        button class=(format!("btn btn--{}", variant.class())) type="button" data-button=(variant.class()) data-action=(action) data-url=(url) { (label) }
    }
}

fn button(variant: ButtonVariant, label: &str, action: &str, body: &str) -> Markup {
    html! {
        button class=(format!("btn btn--{}", variant.class())) type="button" data-button=(variant.class()) data-action=(action) data-modal-title=(label) data-modal-body=(body) { (label) }
    }
}

fn modal_root() -> Markup {
    html! {
        div id="popup-root" data-popup-root="true" {
            div id="modal-overlay" class="modal-overlay" hidden {
                section class="modal-card" role="dialog" aria-modal="true" aria-labelledby="modal-title" {
                    button id="modal-close" class="modal-close" type="button" aria-label="Close modal" { "×" }
                    h2 id="modal-title" class="modal-title" {}
                    div id="modal-content" class="modal-content" {}
                    footer class="modal-actions" { (button(ButtonVariant::Primary, "OK", "modal-ok", "")) }
                }
            }
            div id="toast-container" class="toast-container" aria-live="polite" {}
        }
    }
}
