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
                    header class="app-header" {
                        div class="header-title" {
                            p class="eyebrow" { "Arcadia" }
                            h1 { "HomeConsole" }
                        }
                    }

                    main class="console-pane" aria-label="HomeConsole intent modules" {
                        section class="intent-grid" {
                            article class="intent-module" aria-labelledby="system-status-title" data-module="system-status" {
                                div class="module-head" {
                                    p class="eyebrow" { "Intent module" }
                                    h2 id="system-status-title" { "System status" }
                                    p { "One brick for current machine state and display preference." }
                                }
                                div class="status-strip" {
                                    (status_card("Network", "Online"))
                                    (status_card("Runtime", &status.runtime.machine_uptime))
                                    (status_card("Vault", yes_no(status.vault.mounted)))
                                    div class="status-card status-card--action" { (theme_button()) }
                                }
                            }

                            article class="intent-module intent-module--controls" aria-labelledby="console-controls-title" data-module="console-controls" {
                                div class="module-head" {
                                    p class="eyebrow" { "Intent module" }
                                    h2 id="console-controls-title" { "Console controls" }
                                    p { "One brick for machine-level actions. Each button names the exact object it changes." }
                                }
                                div class="button-row" {
                                    (button(ButtonVariant::Primary, "Reboot console", "reboot-console", "Reboot console restarts the HomeConsole machine deliberately through the governed local path after confirmation and receipt wiring."))
                                    (button(ButtonVariant::Danger, "Shut down console", "shutdown-console", "Shut down console powers the HomeConsole machine down deliberately after confirmation and receipt wiring."))
                                    (button(ButtonVariant::Secondary, "Update console", "update-console", "Update console runs the Harmonia HomeConsole update path and records an update receipt."))
                                }
                            }

                            article class="intent-module" aria-labelledby="vault-module-title" data-module="vault-password" {
                                div class="module-head" {
                                    p class="eyebrow" { "Intent module" }
                                    h2 id="vault-module-title" { "Vault password" }
                                    p { "Change the known Vault password or restore the appliance default. Secret values stay inside the local helper path." }
                                }
                                div class="status-strip" {
                                    (status_card("Vault mounted", yes_no(status.vault.mounted)))
                                    (status_card("Change helper", yes_no(status.vault.password_change_helper_present)))
                                    (status_card("Default reset", yes_no(status.vault.default_reset_available)))
                                }
                                div class="module-grid module-grid--forms" {
                                    article class="submodule control-card" {
                                        h3 { "Change password" }
                                        p { "Use this when the current Vault password is known." }
                                        form id="vault-password-change-form" class="stack" autocomplete="off" {
                                            label { span { "Current password" } input class="field" type="password" name="current_password" autocomplete="current-password" required; }
                                            label { span { "New password" } input class="field" type="password" name="new_password" autocomplete="new-password" required minlength="4"; }
                                            label { span { "Confirm new password" } input class="field" type="password" name="confirm_password" autocomplete="new-password" required minlength="4"; }
                                            div id="vault-password-change-message" class="message" hidden {}
                                            button class="btn btn--primary" type="submit" { "Change Vault password" }
                                        }
                                    }

                                    article class="submodule control-card danger-zone" {
                                        h3 { "Reset to default" }
                                        p { "Use this recovery path only when the appliance default should become the Vault password again." }
                                        form id="vault-password-reset-form" class="stack" autocomplete="off" {
                                            label { span { "Confirmation" } input class="field" type="text" name="confirm" placeholder="RESET" autocomplete="off" required; }
                                            div id="vault-password-reset-message" class="message" hidden {}
                                            button class="btn btn--danger" type="submit" { "Reset Vault password" }
                                        }
                                    }
                                }
                            }

                            article class="intent-module" aria-labelledby="smb-module-title" data-module="smb-uploads" {
                                (smb_explainer(status))
                            }

                            article class="intent-module" aria-labelledby="games-module-title" data-module="games" {
                                div class="module-head" {
                                    p class="eyebrow" { "Intent module" }
                                    h2 id="games-module-title" { "Games" }
                                    p { "Library status, artwork sync, and idle-safe update actions will compose here as their own nested bricks." }
                                }
                            }

                            article class="intent-module" aria-labelledby="receipts-module-title" data-module="receipts" {
                                div class="module-head" {
                                    p class="eyebrow" { "Intent module" }
                                    h2 id="receipts-module-title" { "Receipts" }
                                    p { "Last reboot, update, vault, and health proofs will land here as readable customer receipts." }
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
        div class="module-head" {
            p class="eyebrow" { "Intent module" }
            h2 id="smb-module-title" { "SMB uploads" }
            p { "Copy files to the console over the local home network." }
        }
        div class="text-block" {
            p {
                "Open "
                code { "\\\\" (status.surfaces.smb) }
                " or "
                code { "smb://" (status.surfaces.smb) }
                " from a computer on the same network, then copy files into the shared folder."
            }
            ol class="numbered-list" {
                li { strong { "Connect" } span { "Use File Explorer, Finder, or your Linux file manager and open the SMB address above." } }
                li { strong { "Upload" } span { "Drag ROMs, media, saves, or installer files into the matching share folder." } }
                li { strong { "Wait" } span { "Let the copy finish before unplugging storage, rebooting, or launching the file." } }
            }
            p class="tile-note" {
                "SMB writes across the local network to console storage. Nothing goes to the internet just because SMB is used."
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

fn theme_button() -> Markup {
    html! {
        button id="theme-toggle" class="btn btn--secondary theme-toggle" type="button" aria-label="Switch to dark theme" aria-pressed="false" data-theme-toggle="true" { "Dark" }
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
