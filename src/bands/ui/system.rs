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
                            p { "Fetch the HomeServer certificate bundle automatically, or use a manual bundle when needed." }
                        }
                    }
                    div class="system-ca-state" {
                        (system_field("Mode", if trust.mode == "https" { "HTTPS" } else { "HTTP" }))
                        (system_field("Active bundle", if trust.ca_installed { "Installed" } else { "Empty" }))
                        span class="system-field system-field--anchor" { em { "Anchor" } strong { (trust.ca_path) } }
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
                            div class="inline-actions" { button class="btn btn--secondary" type="submit" { "Install manual bundle" } }
                            div id="root-ca-message" class="message" hidden {}
                        }
                    }
                }
            }
        },
    )
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

