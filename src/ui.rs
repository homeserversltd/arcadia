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
                        }
                        (button(ButtonVariant::Danger, "Restart", "restart-console", "Restart is staged as the only primary control. The reboot action will require confirmation and a receipt before it is wired."))
                    }

                    main class="split-pane" {
                        section class="panel" aria-labelledby="how-console-works" {
                            h2 id="how-console-works" { "How the console works" }
                            p { "HomeConsole is a local appliance. This screen shows the essential status and keeps controls deliberate." }
                            ul class="plain-list" {
                                li { strong { "Network" } span { "Available at console.home.arpa on the local network." } }
                                li { strong { "Runtime" } span { "Machine uptime is " (status.runtime.machine_uptime) ". Arcadia uptime is " (status.runtime.arcadia_uptime) "." } }
                                li { strong { "Control" } span { "Restart is isolated behind one explicit action." } }
                            }
                        }

                        section class="panel" aria-labelledby="status-details" {
                            h2 id="status-details" { "Status details" }
                            (placeholder("Games", "Library status and update-safe idle state."))
                            (placeholder("Vault", &format!("Mounted: {}. Unlock helper: {}.", yes_no(status.vault.mounted), yes_no(status.vault.unlock_helper_present))))
                            (placeholder("Updates", "Harmonia profile state and Arcadia artifact version."))
                            (placeholder("Receipts", "Last restart, health check, and update proof."))
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

fn placeholder(title: &str, body: &str) -> Markup {
    html! {
        article class="detail-row" {
            h3 { (title) }
            p { (body) }
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
