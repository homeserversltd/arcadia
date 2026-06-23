fn system_view(status: &ConsoleStatus) -> Markup {
    let ssh = &status.system.ssh;
    let trust = &status.system.trust;
    view_shell(
        "system",
        "Machine status",
        "System",
        "",
        html! {
            (system_power_panel())
            section class="system-grid system-grid--admin" aria-label="System administration" {
                article class="system-card system-card--ssh" {
                    div class="system-card-band" { strong { "Remote Access" } b class=(status_class(&ssh.service_state)) { (title_case_state(&ssh.service_state)) } }
                    div class="system-field-grid" {
                        (system_field("State", &title_case_state(&ssh.service_state)))
                        (system_field("Login policy", &ssh_login_policy(&ssh.password_auth)))
                        (system_field("Host", &ssh.hostname))
                        (system_field("Address", &ssh.lan_ip))
                        (system_field("User", &ssh.username))
                        (system_field("Keys", &ssh.authorized_keys_path))
                    }
                    (command_box("Connect", &ssh.command))
                    div class="inline-actions" {
                        (ssh_service_action(&ssh.service_state))
                        (ssh_password_action(&ssh.password_auth))
                        (copy_button("Copy Command", &ssh.command))
                    }
                    form id="ssh-key-form" class="settings-form system-compact-form" autocomplete="off" {
                        label { span { "Trusted public key" } textarea class="field field--textarea" name="public_key" rows="3" placeholder="ssh-ed25519 AAAA... homeconsole" {} }
                        div class="inline-actions" { button class="btn btn--primary" type="submit" { "Add Trusted Key" } }
                        div id="ssh-key-message" class="message" hidden {}
                    }
                }

                article class="system-card system-card--trust" {
                    div class="system-card-band" { strong { "Secure Web Access" } b class=(status_class(if trust.mode == "https" { "running" } else { "stopped" })) { @if trust.mode == "https" { "Secure" } @else { "Local HTTP" } } }
                    div class="system-field-grid" {
                        (system_field("Mode", if trust.mode == "https" { "HTTPS with Home Root CA" } else { "Local HTTP" }))
                        (system_field("Home Root CA", if trust.ca_installed { "Installed" } else { "Needed" }))
                        (system_field("Anchor", trust.ca_path))
                        @if let Some(expiry) = trust.ca_not_after.as_deref() { (system_field("Expires", expiry)) }
                    }
                    form id="root-ca-form" class="settings-form system-compact-form" autocomplete="off" {
                        label { span { "Home Root CA bundle" } textarea class="field field--textarea" name="ca_bundle" rows="5" placeholder="-----BEGIN CERTIFICATE-----" {} }
                        div class="inline-actions" { button class="btn btn--primary" type="submit" { "Install Home Root CA" } }
                        div id="root-ca-message" class="message" hidden {}
                    }
                    div class="inline-actions" {
                        (trust_mode_action(&trust.mode))
                    }
                }

                article class="system-card system-card--services" {
                    div class="system-card-band" { strong { "System Health" } b class="system-status system-status--available" { "Observed" } }
                    div class="system-service-list" {
                        @for svc in &status.system.services {
                            (system_service_row_dynamic(&svc.name, &svc.state, &svc.detail))
                        }
                    }
                }

                article class="system-card system-card--logs system-card--desktop-detail" {
                    div class="system-card-band" { strong { "Diagnostics" } b class="system-status system-status--stopped" { "Drill-in" } }
                    p class="card-line" { "Support evidence stays behind this drill-in so the front panel remains an appliance." }
                    div class="inline-actions" {
                        (system_diagnostics_button(status))
                    }
                }
            }
        },
    )
}

fn status_card(title: &str, value: &str, help: &str) -> Markup {
    html! { article class="status-card" { span { (title) } strong { (value) } p { (help) } } }
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

fn command_box(label: &str, value: &str) -> Markup {
    html! {
        div class="command-box" {
            span { (label) }
            code { (value) }
            (copy_button("Copy", value))
        }
    }
}

fn system_power_panel() -> Markup {
    html! {
        section class="system-power-panel" aria-label="Power and sessions" {
            div class="system-power-state" {
                span class="system-power-icon" aria-hidden="true" { "⏻" }
                span { "Power & Sessions" }
                strong { "Front panel" }
            }
            div class="system-power-actions" {
                (system_power_action("Restart Console", "Full system reboot", ButtonVariant::Danger, "reboot-console", "/api/actions/reboot-console"))
                (system_power_action("Shut Down", "Power off appliance", ButtonVariant::Danger, "shutdown-console", "/api/actions/shutdown-console"))
                (system_power_action("Restart Interface", "Web front panel only", ButtonVariant::Secondary, "restart-arcadia", "/api/actions/restart-arcadia"))
                (system_power_action("Restart Game Session", "Game session only", ButtonVariant::Secondary, "restart-gamescope", "/api/actions/restart-gamescope"))
            }
        }
    }
}

fn system_power_action(
    label: &str,
    detail: &str,
    variant: ButtonVariant,
    action: &str,
    endpoint: &str,
) -> Markup {
    html! {
        article class="system-power-action" {
            span { (detail) }
            (action_button(variant, label, action, endpoint))
        }
    }
}

fn copy_button(label: &str, value: &str) -> Markup {
    html! { button class="btn btn--secondary" type="button" data-copy-value=(value) { (label) } }
}

fn ssh_login_policy(password_auth: &str) -> &'static str {
    if matches!(password_auth, "enabled" | "running" | "available") {
        "Password allowed"
    } else {
        "Key login only"
    }
}

fn ssh_service_action(service_state: &str) -> Markup {
    if matches!(service_state, "running" | "enabled" | "available") {
        action_button(
            ButtonVariant::Danger,
            "Turn Off Remote Access",
            "disable-ssh",
            "/api/system/ssh/service",
        )
    } else {
        action_button(
            ButtonVariant::Primary,
            "Turn On Remote Access",
            "enable-ssh",
            "/api/system/ssh/service",
        )
    }
}

fn ssh_password_action(password_auth: &str) -> Markup {
    if matches!(password_auth, "enabled" | "running" | "available") {
        action_button(
            ButtonVariant::Secondary,
            "Require Key Login",
            "disable-ssh-password",
            "/api/system/ssh/password-auth",
        )
    } else {
        action_button(
            ButtonVariant::Secondary,
            "Allow Password Login",
            "enable-ssh-password",
            "/api/system/ssh/password-auth",
        )
    }
}

fn trust_mode_action(mode: &str) -> Markup {
    if mode == "https" {
        action_button(
            ButtonVariant::Secondary,
            "Use Local HTTP",
            "trust-mode-http",
            "/api/system/trust/mode",
        )
    } else {
        action_button(
            ButtonVariant::Primary,
            "Enable Secure Web Access",
            "trust-mode-https",
            "/api/system/trust/mode",
        )
    }
}

fn system_service_row_dynamic(name: &str, status: &str, detail: &str) -> Markup {
    html! {
        div class="system-service-row" {
            span class="system-service-main" { strong { (name) } em { (detail) } }
            b class=(status_class(status)) { (title_case_state(status)) }
        }
    }
}

fn system_diagnostics_button(status: &ConsoleStatus) -> Markup {
    let body = system_diagnostics_body(status);
    modal_button(
        ButtonVariant::Secondary,
        "Open Diagnostics",
        "System Diagnostics",
        &body,
    )
}

fn system_diagnostics_body(status: &ConsoleStatus) -> String {
    let services = status
        .system
        .services
        .iter()
        .map(|svc| {
            format!(
                "{}: {} ({})",
                svc.name,
                title_case_state(&svc.state),
                svc.detail
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Power, access, trust, and service evidence for support.\n\nRemote access: {}\nLogin policy: {}\nWeb access: {}\n\nServices:\n{}",
        title_case_state(&status.system.ssh.service_state),
        ssh_login_policy(&status.system.ssh.password_auth),
        if status.system.trust.mode == "https" { "HTTPS with Home Root CA" } else { "Local HTTP" },
        services
    )
}

fn status_class(status: &str) -> String {
    let class = match status {
        "running" | "enabled" | "available" => "running",
        "starting" => "starting",
        "error" | "failed" => "error",
        _ => "stopped",
    };
    format!("system-status system-status--{}", class)
}

