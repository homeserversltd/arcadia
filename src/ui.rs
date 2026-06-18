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
                    header class="hero-banner" {
                        div class="brand-mark" aria-hidden="true" { "A" }
                        div class="hero-copy" {
                            p class="eyebrow" { "Arcadia / HomeConsole" }
                            h1 { "Console, calm and ready." }
                            p { "One local appliance surface for network presence, runtime state, and operator-controlled transitions." }
                        }
                        div class="hero-action" {
                            (button(ButtonVariant::Danger, "Restart Console", "restart-console", "Restart is intentionally staged here as the single primary control. The wired reboot action will land behind an explicit confirmation receipt."))
                        }
                    }

                    section class="indicator-row" aria-label="Console indicators" {
                        (indicator("Network", "online", status.surfaces.http, "DNS, mDNS, and SMB identity are declared."))
                        (indicator("Machine runtime", "steady", &status.runtime.machine_uptime, "Time since this machine booted."))
                        (indicator("Arcadia runtime", "steady", &status.runtime.arcadia_uptime, "Time since this Arcadia process started."))
                    }

                    main class="two-pane" {
                        section class="pane pane--left" aria-labelledby="how-console-works" {
                            p class="section-label" { "How this console works" }
                            h2 id="how-console-works" { "HomeConsole is a local-first appliance." }
                            p { "Arcadia is the graphical surface. It should explain the machine, show the health of the local route, and expose only deliberate controls." }
                            ol class="console-steps" {
                                li { strong { "Network presence" } span { "The console announces itself as console.home.arpa, homeconsole.local, and HOMECONSOLE." } }
                                li { strong { "Runtime awareness" } span { "The banner keeps machine uptime and Arcadia process uptime visible without visual clutter." } }
                                li { strong { "Manual control" } span { "Dangerous actions stay explicit. Restart begins as one calm button, not a field of controls." } }
                                li { strong { "Receipts next" } span { "Future actions will show proof after the transition rather than spraying buttons before authority exists." } }
                            }
                        }

                        section class="pane pane--right" aria-labelledby="console-placeholders" {
                            p class="section-label" { "Console notes" }
                            h2 id="console-placeholders" { "Placeholders for the next useful facts" }
                            (placeholder("Games", "Library status, active game process, and update-safe idle state will land here."))
                            (placeholder("Vault", &format!("Vault mount: {}. Unlock helper: {}.", yes_no(status.vault.mounted), yes_no(status.vault.unlock_helper_present))))
                            (placeholder("Updates", "Harmonia profile state, Arcadia artifact SHA, and last update receipt will land here."))
                            (placeholder("Receipts", "Last restart, last health check, and last operator action proof will land here."))
                        }
                    }
                }
                (modal_root())
                script src="/static/app.js" {}
            }
        }
    }
}

fn indicator(title: &str, state: &str, value: &str, detail: &str) -> Markup {
    html! {
        article class=(format!("indicator indicator--{}", state)) {
            span { (title) }
            strong { (value) }
            p { (detail) }
        }
    }
}

fn placeholder(title: &str, body: &str) -> Markup {
    html! {
        article class="placeholder-card" {
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
                section class="pane pane--modal" role="dialog" aria-modal="true" aria-labelledby="modal-title" {
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
