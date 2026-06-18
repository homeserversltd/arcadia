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
                    }

                    main class="split-pane" {
                        section class="panel" aria-labelledby="how-console-works" {
                            h2 id="how-console-works" { "How the console works" }
                            p { "HomeConsole is a local appliance. This screen shows the essential status and keeps controls deliberate." }
                            div class="power-controls" aria-label="Console power controls" {
                                (button(ButtonVariant::Primary, "Start", "start-console", "Start wakes or launches the console runtime through the governed HomeConsole path."))
                                (button(ButtonVariant::Danger, "Shut down", "shutdown-console", "Shut down powers the console down deliberately after confirmation and receipt wiring."))
                                (button(ButtonVariant::Secondary, "Update", "update-console", "Update runs the Harmonia HomeConsole update path and records an update receipt."))
                            }
                            ul class="plain-list" {
                                li { strong { "Network" } span { "Available at console.home.arpa on the local network." } }
                                li { strong { "Runtime" } span { "Machine uptime is " (status.runtime.machine_uptime) ". Arcadia uptime is " (status.runtime.arcadia_uptime) "." } }
                                li { strong { "Control" } span { "Start, shut down, and update live in the left pane as deliberate appliance controls." } }
                            }
                        }

                        section class="panel" aria-labelledby="status-details" {
                            h2 id="status-details" { "Status details" }
                            (smb_explainer(status))
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

fn smb_explainer(status: &ConsoleStatus) -> Markup {
    html! {
        article class="detail-row smb-guide" aria-labelledby="smb-guide-title" {
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
