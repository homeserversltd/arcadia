fn vault_view(status: &ConsoleStatus) -> Markup {
    let mounted = status.vault.mounted;
    let auto_decrypt = status.vault.auto_decrypt_enabled;
    view_shell(
        "vault",
        "Secure storage",
        "Vault",
        "Choose how the Vault opens when the console starts.",
        html! {
            div class="vault-panel" {
                article class="access-pin-card access-pin-card--mode" data-module="vault-access" {
                    div class="access-pin-state" { span class="access-pin-icon" aria-hidden="true" { (if mounted { "●" } else { "○" }) } div { h3 { "Vault status" } strong data-vault-mode-label="true" { (if mounted { "Unlocked" } else { "Locked" }) } p data-vault-mode-copy="true" { (if auto_decrypt { "Automatically decrypts when the console starts." } else { "Unlock required after the console starts." }) } } }
                    label class="pin-toggle vault-toggle" { input type="checkbox" role="switch" name="auto_decrypt_enabled" data-vault-auto-decrypt-toggle="true" checked[auto_decrypt]; span class="pin-toggle-track" aria-hidden="true" { span class="pin-toggle-thumb" {} } span class="pin-toggle-label" { "Automatically decrypt at boot" } }
                }
                @if !mounted { (settings_action_row("Unlock now", "Enter the Vault password to unlock it for this session.", Some("●"), Some(("Vault locked", "system-status--disabled")), html! { form id="vault-settings-unlock-form" class="settings-form settings-form--vault" autocomplete="off" { input class="field" type="password" name="password" placeholder="Vault password" autocomplete="current-password" required; button class="btn btn--primary" type="submit" { "Unlock Vault" } div id="vault-settings-unlock-message" class="message" hidden {} } }, false)) }
            }
        },
    )
}
