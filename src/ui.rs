use maud::{html, Markup, DOCTYPE};

use crate::{ButtonVariant, ConsoleStatus};

pub fn layout(status: &ConsoleStatus) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (status.product) " / Arcadia" }
                link rel="stylesheet" href="/static/app.css";
            }
            body data-ui-schema=(status.ui_contract.schema) {
                div id="app" class="console-shell" {
                    header class="app-header" {
                        div class="header-title" {
                            p class="eyebrow" { "Arcadia" }
                            h1 { "HomeConsole" }
                        }
                        div class="header-status" aria-label="Console indicators" {
                            (indicator("Network", "Online"))
                            (indicator("Runtime", &status.runtime.machine_uptime))
                            (indicator("Vault", yes_no(status.vault.mounted)))
                            (theme_button())
                        }
                    }

                    main class="appliance-layout" {
                        aside class="left-pane" aria-label="Arcadia appliance tiles" {
                            article class="control-cluster" aria-label="Console power controls" {
                                h2 { "Console controls" }
                                div class="power-controls" {
                                    (button(ButtonVariant::Primary, "Reboot console", "reboot-console", "Reboot console restarts the HomeConsole machine deliberately through the governed local path after confirmation and receipt wiring."))
                                    (button(ButtonVariant::Danger, "Shut down", "shutdown-console", "Shut down powers the console down deliberately after confirmation and receipt wiring."))
                                    (button(ButtonVariant::Secondary, "Update", "update-console", "Update runs the Harmonia HomeConsole update path and records an update receipt."))
                                }
                            }
                            (nav_tile("Vault Password", "Change or reset the Vault appliance password", "vault-password", "🔐", true, ButtonVariant::Primary))
                            (nav_tile("SMB Uploads", "How local file sharing works", "smb-uploads", "⇄", false, ButtonVariant::Secondary))
                            (nav_tile("Games", "Library and idle-safe updates", "games", "🎮", false, ButtonVariant::Secondary))
                            (nav_tile("Receipts", "Last local proof readbacks", "receipts", "🧾", false, ButtonVariant::Secondary))
                        }

                        section class="management-pane" aria-labelledby="vault-password-title" data-panel="vault-password" {
                            div class="panel-head" {
                                p class="eyebrow" { "Vault system" }
                                h2 id="vault-password-title" { "Vault Password" }
                                p { "Manage the Vault password like a router appliance: change it when known, or reset it to the HomeConsole default from this front end." }
                            }

                            div class="status-strip" {
                                (status_card("Vault mounted", yes_no(status.vault.mounted)))
                                (status_card("Change helper", yes_no(status.vault.password_change_helper_present)))
                                (status_card("Default reset", yes_no(status.vault.default_reset_available)))
                            }

                            div class="form-grid" {
                                article class="card control-card" {
                                    h3 { "Change password" }
                                    p { "Enter the current Vault password and the replacement password. Arcadia sends secrets only to the local helper stdin; responses stay redacted." }
                                    form id="vault-password-change-form" class="stack" autocomplete="off" {
                                        label { span { "Current password" } input class="field" type="password" name="current_password" autocomplete="current-password" required; }
                                        label { span { "New password" } input class="field" type="password" name="new_password" autocomplete="new-password" required minlength="4"; }
                                        label { span { "Confirm new password" } input class="field" type="password" name="confirm_password" autocomplete="new-password" required minlength="4"; }
                                        div id="vault-password-change-message" class="message" hidden {}
                                        button class="btn btn--primary" type="submit" { "Change Vault Password" }
                                    }
                                }

                                article class="card control-card danger-zone" {
                                    h3 { "Reset to default" }
                                    p { "Restore the Vault password to the appliance default for recovery. Type RESET to confirm the router-style reset action." }
                                    form id="vault-password-reset-form" class="stack" autocomplete="off" {
                                        label { span { "Confirmation" } input class="field" type="text" name="confirm" placeholder="RESET" autocomplete="off" required; }
                                        div id="vault-password-reset-message" class="message" hidden {}
                                        button class="btn btn--danger" type="submit" { "Reset to Default" }
                                    }
                                }
                            }

                            (smb_explainer(status))
                        }
                    }
                }
                (modal_root())
                script src="/static/app.js" {}
            }
        }
    }
}

fn indicator(title: &str, value: &str) -> Markup {
    html! {
        div class="status-pill" {
            span { (title) }
            strong { (value) }
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

fn nav_tile(
    title: &str,
    body: &str,
    action: &str,
    icon: &str,
    active: bool,
    variant: ButtonVariant,
) -> Markup {
    let class = if active {
        "nav-tile nav-tile--active"
    } else {
        "nav-tile"
    };
    html! {
        button class=(class) type="button" data-nav-action=(action) data-button=(variant.class()) {
            span class="nav-icon" aria-hidden="true" { (icon) }
            span class="nav-copy" { strong { (title) } small { (body) } }
        }
    }
}

fn smb_explainer(status: &ConsoleStatus) -> Markup {
    html! {
        article class="smb-guide" aria-labelledby="smb-guide-title" {
            p class="tile-kicker" { "File uploads" }
            h3 id="smb-guide-title" { "How SMB file sharing works" }
            p {
                "SMB is the normal Windows-style network file share for this console. When the console and your computer are on the same home network, open "
                code { "\\\\" (status.surfaces.smb) }
                " or "
                code { "smb://" (status.surfaces.smb) }
                ", sign in if prompted, then copy files into the shared folder."
            }
            ol class="numbered-list" {
                li { strong { "Connect" } span { "Use File Explorer, Finder, or your Linux file manager and open the SMB address above." } }
                li { strong { "Upload" } span { "Drag ROMs, media, saves, or installer files into the matching share folder; the copy dialog is the upload progress." } }
                li { strong { "Let it finish" } span { "Wait for the copy to complete before unplugging storage, restarting, or launching the file." } }
            }
            p class="tile-note" {
                "Under the hood, the console is running a local Samba service. It publishes a folder on the LAN; your computer writes the file across the network; the file lands on the console disk or vault-backed storage; Arcadia then reads from that local storage. Nothing goes to the internet just because you used SMB."
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

fn theme_button() -> Markup {
    html! {
        button id="theme-toggle" class="btn btn--secondary theme-toggle" type="button" aria-label="Switch color theme" aria-pressed="false" data-theme-toggle="true" { "Dark" }
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
