fn system_view(status: &ConsoleStatus) -> Markup {
    let ssh = &status.system.ssh;
    let trust = &status.system.trust;
    view_shell(
        "system",
        "System",
        "System",
        "",
        html! {
            section class="system-redo" aria-label="System controls" {
                article class="system-button-tile" aria-label="Console actions" {
                    (action_button(ButtonVariant::Danger, "Restart Console", "reboot-console", "/api/actions/reboot-console"))
                    (action_button(ButtonVariant::Danger, "Shutdown", "shutdown-console", "/api/actions/shutdown-console"))
                    (action_button(ButtonVariant::Secondary, "Restart GameScope", "restart-gamescope", "/api/actions/restart-gamescope"))
                    (action_button(ButtonVariant::Secondary, "Restart Arcadia", "restart-arcadia", "/api/actions/restart-arcadia"))
                    (passwordless_ssh_toggle(&ssh.service_state))
                }

                article class="system-ca-panel" aria-label="Certificate bundle ingestion" data-active-bundle=(if trust.ca_installed { "present" } else { "missing" }) {
                    div class="system-ca-hero" {
                        div class="system-ca-orb" aria-label="HTTPS mode" {
                            span { "HTTPS" }
                            strong { (if trust.mode == "https" { "ON" } else { "HTTP" }) }
                        }
                        div class="system-ca-copy" {
                            strong { "Household trust" }
                            p { "This console binds the HomeServer certificate on its own the first time it meets it." }
                        }
                    }
                    section class="household-trust-card" data-household-trust="true" aria-label="Household trust" {
                        div class="household-trust-card__head" {
                            span { "Household trust" }
                            b class=(format!("system-status system-status--{}", if !trust.caduceus_available { "error" } else if trust.ca_installed { "available" } else { "unknown" })) data-household-trust-state title=(trust.ring_fingerprint.as_deref().unwrap_or("")) {
                                (if !trust.caduceus_available { "Unavailable" } else if trust.ca_installed { "Bound" } else { "Waiting for HomeServer" })
                            }
                        }
                        div class="household-trust-card__fields" {
                            span class="system-field" { em { "Ring fingerprint" } strong data-household-trust-fingerprint title=(trust.ring_fingerprint.as_deref().unwrap_or("")) { (trust.ring_fingerprint.as_deref().map(|fingerprint| fingerprint.chars().take(20).collect::<String>()).unwrap_or_else(|| "—".to_string())) } }
                            span class="system-field" { em { "Bound to" } strong data-household-trust-gateway { (if trust.caduceus_available && trust.ca_installed { status.network.gateway.as_deref().unwrap_or("—") } else { "—" }) } }
                        }
                        div class="household-trust-renewal" data-household-trust-renewal hidden {}
                        p class="household-trust-card__error" data-household-trust-error hidden {}
                    }
                    div class="system-ca-state" {
                        (system_field("Mode", if trust.mode == "https" { "HTTPS" } else { "HTTP" }))
                        (system_field("Active bundle", if trust.ca_installed { "Installed" } else { "Empty" }))
                        @if let Some(path) = trust.ca_path { span class="system-field system-field--anchor" { em { "Anchor" } strong { (path) } } }
                        @if let Some(expiry) = trust.ca_not_after.as_deref() { (system_field("Expires", expiry)) }
                    }
                    details class="system-manual-install" {
                        summary { "Manual install" }
                        form id="root-ca-form" class="settings-form system-ca-form" autocomplete="off" enctype="multipart/form-data" {
                            label class="system-upload-drop" {
                                input type="file" name="ca_bundle_file" accept=".pem,.crt,.cer,.bundle,.chain,text/plain,application/x-pem-file,application/pem-certificate-chain" {}
                                span class="system-upload-face" {
                                    strong { "Certificate bundle" }
                                    em { "Select PEM / CRT file" }
                                }
                            }
                            label class="system-ca-paste" {
                                span { "Paste PEM bundle" }
                                textarea class="field field--textarea" name="ca_bundle" rows="5" placeholder="-----BEGIN CERTIFICATE-----" {}
                            }
                            div class="inline-actions" { button class="btn btn--secondary" type="submit" { "Upload CA Bundle" } }
                            div id="root-ca-message" class="message" hidden {}
                        }
                    }
                }
                (system_access_section(status))
            }
        },
    )
}

fn system_access_section(status: &ConsoleStatus) -> Markup {
    let mounted = status.vault.mounted;
    let auto_decrypt = status.vault.auto_decrypt_enabled;
    html! {
        article class="system-access-panel" aria-label="Locks" data-module="caduceus-attendance-access" {
            header class="system-access-panel__head" { span { "Locks" } strong data-admin-projection="true" data-bind="systemPane.adminText" { "Guest" } }
            div class="system-access-stack" {
                @if status.vault.present {
                    article class="access-pin-card access-pin-card--mode system-access-card" data-module="vault-access" data-bind-class="status.vault.mounted" {
                        div class="access-pin-state" {
                            span class="access-pin-icon" aria-hidden="true" { (if mounted { "●" } else { "○" }) }
                            div {
                                h3 { "1 · At boot" }
                                strong data-bind="systemPane.vaultLabel" { (if mounted { "Unlocked" } else { "Locked" }) }
                                p data-bind="systemPane.vaultCopy" { (if auto_decrypt { "Automatically decrypts when the console starts." } else { "Asks for the vault password at boot." }) }
                            }
                        }
                        label class="pin-toggle vault-toggle" {
                            input type="checkbox" role="switch" name="auto_decrypt_enabled" data-vault-auto-decrypt-toggle="true" data-bind-checked="status.vault.auto_decrypt_enabled" checked[auto_decrypt];
                            span class="pin-toggle-track" aria-hidden="true" { span class="pin-toggle-thumb" {} }
                            span class="pin-toggle-label" { "Unlock vault automatically" }
                        }
                    }
                }
                article class="access-pin-card access-pin-card--mode system-access-card" data-module="gui-pin-access" data-bind-class="status.gui_pin.pin_required" {
                    div class="access-pin-state" {
                        span class="access-pin-icon" aria-hidden="true" { "●" }
                        div {
                            h3 { "2 · At console" }
                            strong data-bind="systemPane.pinLabel" { (if status.gui_pin.pin_required { "PIN required" } else { "PIN not required" }) }
                            p data-bind="systemPane.pinCopy" { "Caduceus attendance controls console administration." }
                        }
                    }
                    label class="pin-toggle" {
                        input type="checkbox" role="switch" name="pin_required" data-pin-required-toggle="true" data-bind-checked="status.gui_pin.pin_required" checked[status.gui_pin.pin_required];
                        span class="pin-toggle-track" aria-hidden="true" { span class="pin-toggle-thumb" {} }
                        span class="pin-toggle-label" { "Require PIN" }
                    }
                }
                details class="access-pin-card access-pin-card--change system-access-card" {
                    summary { "Change PIN" }
                    form id="gui-pin-change-form" class="settings-form settings-form--pin" autocomplete="off" {
                        label { span { "Current PIN" } input class="field" type="password" name="current_pin" autocomplete="current-password" required; }
                        div class="pin-form-row" {
                            label { span { "New PIN" } input class="field" type="password" name="new_pin" autocomplete="new-password" required minlength="4"; }
                            label { span { "Confirm new PIN" } input class="field" type="password" name="confirm_pin" autocomplete="new-password" required minlength="4"; }
                        }
                        div id="gui-pin-change-message" class="message" hidden {}
                        button class="btn btn--primary" type="submit" { "Change PIN" }
                    }
                }
                details class="access-pin-card access-pin-card--reset system-access-card" {
                    summary { "Reset to default" }
                    (settings_action_row("Reset to default PIN", "Caduceus restores the appliance default; games, settings, storage, and the operating system stay unchanged.", Some("!"), Some(("Caduceus action", "system-status--ok")), html! { button class="btn btn--danger" type="button" data-gui-pin-reset-default="true" { "Reset to default" } div id="gui-pin-reset-message" class="message" hidden {} }, true))
                }
            }
        }
    }
}

fn storage_stat(label: &str, value: &str) -> Markup {
    html! { div class="storage-stat" { span { (label) } strong { (value) } } }
}

fn model_meta(label: &str, value: &str) -> Markup {
    html! { span class="model-meta" { em { (label) } strong { (value) } } }
}

fn system_field(label: &str, value: &str) -> Markup {
    html! { span class="system-field" { em { (label) } strong { (value) } } }
}

fn passwordless_ssh_toggle(service_state: &str) -> Markup {
    if matches!(service_state, "running" | "enabled" | "available") {
        action_button(
            ButtonVariant::Secondary,
            "Turn Off Passwordless SSH",
            "disable-ssh",
            "/api/system/ssh/service",
        )
    } else {
        action_button(
            ButtonVariant::Primary,
            "Turn On Passwordless SSH",
            "enable-ssh",
            "/api/system/ssh/service",
        )
    }
}

fn copy_button(label: &str, value: &str) -> Markup {
    html! { button class="btn btn--secondary" type="button" data-copy-value=(value) { (label) } }
}
