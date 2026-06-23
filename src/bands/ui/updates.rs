fn updates_view(status: &ConsoleStatus) -> Markup {
    view_shell(
        "updates",
        "",
        "",
        "",
        html! {
            div class="harmonia-panel" data-harmonia-updates="true" {
                article class=(format!("harmonia-currentness harmonia-currentness--{}", status.updates.state)) {
                    div class="harmonia-orb" aria-hidden="true" { "H" }
                    div class="harmonia-currentness-copy" {
                        strong { (harmonia_state_label(&status.updates.state)) }
                        span { (status.updates.profile_id) " / " (status.updates.identity) " · " (status.updates.first_missing_signal) }
                    }
                    div class="harmonia-currentness-actions" {
                        (action_button(ButtonVariant::Secondary, "Check state", "check-updates", "/api/actions/check-updates"))
                        (action_button(ButtonVariant::Primary, "Make harmonious", "update-gui", "/api/actions/update-gui"))
                        button class="btn btn--secondary" type="button" data-harmonia-module-menu="true" { "Modules" }
                        button class="btn btn--secondary" type="button" data-harmonia-ledger-open="true" { "Ledger" }
                    }
                }
                div class="harmonia-metrics" {
                    (status_card("Arcadia", status.arcadia.version, "GUI runtime"))
                    (status_card("Modules", &format!("{} enabled", status.updates.modules.iter().filter(|module| module.enabled).count()), "Profile spine"))
                    (status_card("Operations", &status.updates.operation_count.to_string(), "Last Harmonia run"))
                    (status_card("Receipt", &receipt_short_name(&status.updates.latest_receipt), "Suite receipt"))
                }
                div class="harmonia-module-grid" data-harmonia-module-grid="true" {
                    @for module in &status.updates.modules {
                        (harmonia_module_row(module))
                    }
                }
                details class="collapsible-log harmonia-receipts" {
                    summary { "Receipts" }
                    div class="system-field-grid" {
                        (system_field("Suite", &status.updates.latest_receipt))
                        (system_field("Check", &status.updates.latest_check_receipt))
                        (system_field("Module root", &status.updates.module_root))
                    }
                }
                div id="console-action-message" class="message" hidden {}
            }
        },
    )
}

fn harmonia_module_readiness(modules: &[crate::HarmoniaModuleStatus]) -> (usize, usize) {
    let enabled = modules.iter().filter(|module| module.enabled).count();
    let ready = modules
        .iter()
        .filter(|module| module.enabled && module.present)
        .count();
    (ready, enabled)
}

fn harmonia_check_status_label(status: &crate::UpdatesStatus) -> String {
    if status.check_missing_signal == "not-checked" {
        "Not run".to_string()
    } else if status.check_ok {
        "Current".to_string()
    } else {
        "Needs repair".to_string()
    }
}

fn harmonia_suite_status_label(status: &crate::UpdatesStatus) -> String {
    if status.suite_ok {
        "Current".to_string()
    } else if status.first_missing_signal == "receipt-missing" {
        "Not run".to_string()
    } else {
        "Needs repair".to_string()
    }
}

fn harmonia_missing_signal_label(signal: &str) -> String {
    match signal {
        "none" => "None".to_string(),
        "not-checked" => "Not checked yet".to_string(),
        "receipt-missing" => "No Harmonia receipt".to_string(),
        other => other.replace('-', " "),
    }
}

fn harmonia_state_label(state: &str) -> &'static str {
    match state {
        "current" => "Harmonia current",
        "repair_pending" => "Repair pending",
        "checking" => "Checking",
        "installing" => "Making harmonious",
        "available" => "Update available",
        "error" => "Harmonia error",
        _ => "Harmonia unknown",
    }
}

fn receipt_short_name(path: &str) -> String {
    path.rsplit('/')
        .take(2)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("/")
}

fn harmonia_module_row(module: &crate::HarmoniaModuleStatus) -> Markup {
    html! {
        article class=(format!("harmonia-module harmonia-module--{}", module.state)) data-harmonia-module=(module.id) data-module-enabled=(module.enabled) {
            div {
                strong { (module.label) }
                span { (module.id) }
            }
            b class=(format!("system-status system-status--{}", if module.enabled && module.present { "available" } else if module.enabled { "error" } else { "disabled" })) {
                (if module.enabled { if module.present { "Enabled" } else { "Missing" } } else { "Off" })
            }
            button class="btn btn--secondary" type="button" data-harmonia-module-toggle=(module.id) data-enabled=(module.enabled) {
                (if module.enabled { "Turn off" } else { "Turn on" })
            }
        }
    }
}

