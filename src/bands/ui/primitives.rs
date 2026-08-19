fn nav_focus_button(label: &str, target: &str, focus: &str) -> Markup {
    html! { button class="btn btn--secondary" type="button" data-nav-target=(target) data-focus-target=(focus) { (label) } }
}

fn action_button(variant: ButtonVariant, label: &str, action: &str, endpoint: &str) -> Markup {
    html! { button class=(format!("btn btn--{}", variant.class())) type="button" data-button=(variant.class()) data-action=(action) data-endpoint=(endpoint) { (label) } }
}

fn nav_button(label: &str, view: &str) -> Markup {
    html! { button class="btn btn--secondary" type="button" data-nav-target=(view) { (label) } }
}

fn modal_button(
    variant: ButtonVariant,
    label: &str,
    observation_action: &str,
    title: &str,
    body: &str,
) -> Markup {
    html! { button class=(format!("btn btn--{}", variant.class())) type="button" data-button=(variant.class()) data-observation-action=(observation_action) data-modal-title=(title) data-modal-body=(body) { (label) } }
}

fn gui_pin_gate(status: &ConsoleStatus) -> Markup {
    html! {
        section id="gui-pin-gate" class="pin-auth-container" data-required=(status.gui_pin.pin_required) {
            article class="pin-auth-card" {
                div class="product-mark" { "H" }
                h1 { "HomeConsole" }
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

fn vault_unlock_gate(status: &ConsoleStatus) -> Markup {
    html! { section id="vault-unlock-gate" class="vault-auth-container" data-required=(status.vault.unlock_required) { article class="vault-auth-card" { div class="product-mark" { "H" } h1 { "HomeConsole" } h2 { "Vault" } p { "Unlock the vault to continue to HomeConsole." } form id="vault-unlock-form" class="settings-form" autocomplete="off" { input class="field field--vault-password" type="password" name="password" placeholder="Vault password" autocomplete="current-password" autofocus; div id="vault-auth-error" class="message message--error" hidden {} button class="btn btn--primary" type="submit" { "Unlock Vault" } } } } }
}

fn theme_boot_script() -> PreEscaped<&'static str> {
    PreEscaped(
        r#"(function(){try{document.documentElement.dataset.theme=localStorage.getItem('arcadia-theme')||'ember-aubergine';}catch(_){document.documentElement.dataset.theme='ember-aubergine';}})();"#,
    )
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
