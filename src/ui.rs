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
            body data-ui-schema=(status.ui_contract.schema) {
                div id="app" class="console-shell" {
                    main class="console-pane" aria-label="HomeConsole" {
                        section class="control-grid" {
                            article class="panel" data-module="status" {
                                div class="status-strip" {
                                    (status_card("Network", "Online"))
                                    (status_card("Runtime", &status.runtime.machine_uptime))
                                    (status_card("Vault", yes_no(status.vault.mounted)))
                                }
                            }

                            article class="panel panel--controls" data-module="actions" {
                                div class="button-row button-row--five" {
                                    (button(ButtonVariant::Primary, "Update GUI", "update-gui", "Update GUI installs the latest Arcadia build on this console."))
                                    (button(ButtonVariant::Primary, "Sync games", "sync-games", "Sync games reads the game folders, refreshes shortcuts, and adds artwork when keys are present."))
                                    (button(ButtonVariant::Secondary, "Add games", "add-games", "Add games by copying files to the SMB folders shown on this page, then press Sync games."))
                                    (button(ButtonVariant::Secondary, "Reboot console", "reboot-console", "Reboot console restarts this HomeConsole."))
                                    (button(ButtonVariant::Danger, "Shut down console", "shutdown-console", "Shut down console powers this HomeConsole off."))
                                }
                            }

                            article class="panel" data-module="smb-folders" {
                                (smb_explainer(status))
                            }

                            article class="panel" data-module="provider-keys" {
                                p class="tile-note" { "Provider keys let Sync games fetch artwork and metadata. Values are saved for the console and are not shown back." }
                                form id="provider-keys-form" class="stack" autocomplete="off" {
                                    label { span { "SteamGridDB API key" } input class="field" type="password" name="steamgriddb_api_key" autocomplete="off"; }
                                    label { span { "TheGamesDB API key" } input class="field" type="password" name="thegamesdb_api_key" autocomplete="off"; }
                                    label { span { "ScreenScraper user" } input class="field" type="text" name="screenscraper_user" autocomplete="off"; }
                                    label { span { "ScreenScraper password" } input class="field" type="password" name="screenscraper_password" autocomplete="off"; }
                                    div id="provider-keys-message" class="message" hidden {}
                                    button class="btn btn--primary" type="submit" { "Save API keys" }
                                }
                            }

                            article class="panel" data-module="vault-password" {
                                div class="module-grid module-grid--forms" {
                                    article class="submodule control-card" {
                                        p { "Change the vault password when the current password is known." }
                                        form id="vault-password-change-form" class="stack" autocomplete="off" {
                                            label { span { "Current password" } input class="field" type="password" name="current_password" autocomplete="current-password" required; }
                                            label { span { "New password" } input class="field" type="password" name="new_password" autocomplete="new-password" required minlength="4"; }
                                            label { span { "Confirm new password" } input class="field" type="password" name="confirm_password" autocomplete="new-password" required minlength="4"; }
                                            div id="vault-password-change-message" class="message" hidden {}
                                            button class="btn btn--primary" type="submit" { "Change Vault password" }
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

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn theme_boot_script() -> PreEscaped<&'static str> {
    PreEscaped(
        r#"(function(){try{var t=localStorage.getItem('arcadia-theme');if(t!=='light'&&t!=='dark'){t=(window.matchMedia&&window.matchMedia('(prefers-color-scheme: dark)').matches)?'dark':'light';}document.documentElement.dataset.theme=t;}catch(_){document.documentElement.dataset.theme='light';}})();"#,
    )
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
