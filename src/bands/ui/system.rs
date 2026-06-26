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
                    div class="system-ca-copy" {
                        strong { "HTTPS bundle" }
                        p { "Upload one PEM certificate or CA bundle for the active HomeConsole trust slot. A technician can validate the bundle here; once the bundle is secured, Arcadia can switch the console from local HTTP to HTTPS. Only one bundle is active at a time." }
                    }
                    div class="system-ca-state" {
                        (system_field("Mode", if trust.mode == "https" { "HTTPS" } else { "HTTP" }))
                        (system_field("Active bundle", if trust.ca_installed { "Installed" } else { "Empty" }))
                        (system_field("Anchor", trust.ca_path))
                        @if let Some(expiry) = trust.ca_not_after.as_deref() { (system_field("Expires", expiry)) }
                    }
                    form id="root-ca-form" class="settings-form system-ca-form" autocomplete="off" enctype="multipart/form-data" {
                        label class="system-upload-drop" {
                            span { "Certificate bundle" }
                            input class="field" type="file" name="ca_bundle_file" accept=".pem,.crt,.cer,.bundle,.chain,text/plain,application/x-pem-file,application/pem-certificate-chain" {}
                        }
                        label class="system-ca-paste" {
                            span { "Paste PEM bundle" }
                            textarea class="field field--textarea" name="ca_bundle" rows="6" placeholder="-----BEGIN CERTIFICATE-----" {}
                        }
                        div class="inline-actions" { button class="btn btn--primary" type="submit" { "Upload CA Bundle" } }
                        div id="root-ca-message" class="message" hidden {}
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

