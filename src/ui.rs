use maud::{html, Markup, DOCTYPE};

use crate::{ButtonAction, ButtonVariant, ConsoleStatus, Portal, TileDatum};

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
            body data-vault-mounted=(status.vault.mounted) data-ui-schema=(status.ui_contract.schema) {
                (vault_gate(status))
                div id="app" class="surface surface--portals" aria-hidden=(!status.vault.mounted) {
                    (pane("Arcadia", "HomeConsole portals", html! {
                        div class="surface-actions" {
                            (button(ButtonVariant::Secondary, "Vault status", "vault-status", &vault_modal_text(status)))
                            (button(ButtonVariant::Secondary, "UI contract", "ui-contract", status.ui_contract.composition))
                        }
                    }))
                    main class="tile-grid tile-grid--portal" {
                        @for portal in &status.portals { (portal_tile(portal)) }
                    }
                }
                (modal_root())
                script src="/static/app.js" {}
            }
        }
    }
}

fn vault_gate(status: &ConsoleStatus) -> Markup {
    html! {
        section id="vault-gate" class="vault-auth-container" data-mounted=(status.vault.mounted) {
            article class="tile tile--vault-auth" {
                div class="vault-auth-logo" aria-hidden="true" { "A" }
                h1 { "HomeConsole" }
                @if status.vault.mounted {
                    h2 { "Vault Mounted" }
                    p class="tile-copy" { "The vault is unlocked. Opening Arcadia portals." }
                } @else {
                    h2 { "Vault Authentication" }
                    p class="tile-copy" { "Please enter your vault password to continue." }
                    form id="vault-unlock-form" class="stack" autocomplete="off" {
                        input class="field field--password" type="password" name="password" placeholder="Enter vault password" autocomplete="current-password" autofocus;
                        div id="vault-auth-error" class="message message--error" hidden {}
                        (submit_button(ButtonVariant::Primary, "Unlock Vault"))
                    }
                }
                small { "Product of HOMESERVER LLC" }
                small class="mono" { "Version " (env!("CARGO_PKG_VERSION")) " / " (status.arcadia.mode) }
            }
        }
    }
}

fn pane(title: &str, subtitle: &str, tools: Markup) -> Markup {
    html! {
        header class="pane pane--header" {
            div class="pane-copy" { h1 { (title) } p { (subtitle) } }
            (tools)
        }
    }
}

fn portal_tile(portal: &Portal) -> Markup {
    html! {
        article class=(format!("tile tile--portal {}", portal.status.class())) data-tile=(portal.name) data-action=(portal.action) data-url=(portal.local_url) data-modal-title=(portal.name) data-modal-body=(portal_detail(portal)) tabindex="0" role="button" {
            div class="tile-head" {
                div class="tile-icon" aria-hidden="true" { (portal.icon) }
                div { h2 { (portal.name) } p class="tile-copy" { (portal.description) } }
            }
            div class="tile-density" {
                @for datum in &portal.density { (micro_tile(datum)) }
            }
            div class="tile-actions" {
                @for action in &portal.buttons { (button_action(action, portal)) }
            }
        }
    }
}

fn micro_tile(datum: &TileDatum) -> Markup {
    html! {
        div class=(format!("tile tile--micro {}", datum.state.class())) {
            span { (datum.label) }
            strong { (datum.value) }
        }
    }
}

fn button_action(action: &ButtonAction, portal: &Portal) -> Markup {
    html! {
        button class=(format!("btn btn--{}", action.variant.class())) type="button" data-button=(action.variant.class()) data-action=(action.action) data-modal-title=(format!("{} / {}", portal.name, action.label)) data-modal-body=(portal_detail(portal)) {
            (action.label)
        }
    }
}

fn button(variant: ButtonVariant, label: &str, action: &str, body: &str) -> Markup {
    html! {
        button class=(format!("btn btn--{}", variant.class())) type="button" data-button=(variant.class()) data-action=(action) data-modal-title=(label) data-modal-body=(body) { (label) }
    }
}

fn submit_button(variant: ButtonVariant, label: &str) -> Markup {
    html! { button class=(format!("btn btn--{}", variant.class())) type="submit" data-button=(variant.class()) { (label) } }
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

fn vault_modal_text(status: &ConsoleStatus) -> String {
    format!(
        "mounted: {}\nmountpoint: {}\nmapper_present: {}\nunlock_helper_present: {}\nstate_path: {}",
        status.vault.mounted, status.vault.mountpoint, status.vault.mapper_present, status.vault.unlock_helper_present, status.vault.state_path
    )
}

fn portal_detail(portal: &Portal) -> String {
    let dense = portal
        .density
        .iter()
        .map(|d| format!("{}: {}", d.label, d.value))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "{}\n\nAction: {}\n\n{}",
        portal.description, portal.action, dense
    )
}
