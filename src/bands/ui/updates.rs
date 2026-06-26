fn updates_view(status: &ConsoleStatus) -> Markup {
    view_shell(
        "updates",
        "",
        "",
        "",
        html! {
            div class="harmonia-panel harmonia-panel--update-center" data-harmonia-updates="true" {
                div class="harmonia-command-rail" data-harmonia-update-controls="true" {
                    (action_button(ButtonVariant::Primary, "Sync", "update-gui", "/api/actions/update-gui"))
                    button class="btn btn--secondary" type="button" data-harmonia-ledger-open="true" { "Ledger" }
                    (action_button(ButtonVariant::Secondary, "Check state", "check-updates", "/api/actions/check-updates"))
                }
                div class="harmonia-default-grid" data-harmonia-default-grid="true" {
                    article class="harmonia-module-pane" data-harmonia-module-pane="true" {
                        div class="harmonia-pane-chrome" {
                            strong { "Modules" }
                            span { (harmonia_module_toggle_line(&status.updates.modules)) }
                        }
                        div class="harmonia-module-grid" data-harmonia-module-grid="true" {
                            @for module in &status.updates.modules {
                                (harmonia_module_row(module))
                            }
                        }
                    }
                    (harmonia_update_availability_pane(&status.updates))
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

fn harmonia_pending_label(count: usize) -> String {
    if count == 0 {
        "None".to_string()
    } else if count == 1 {
        "1 update".to_string()
    } else {
        format!("{count} updates")
    }
}

fn harmonia_module_readiness(modules: &[crate::HarmoniaModuleStatus]) -> (usize, usize) {
    let enabled = modules.iter().filter(|module| module.enabled).count();
    let ready = modules
        .iter()
        .filter(|module| module.enabled && module.present)
        .count();
    (ready, enabled)
}

fn harmonia_module_toggle_line(modules: &[crate::HarmoniaModuleStatus]) -> String {
    let on = modules.iter().filter(|module| module.enabled).count();
    let off = modules.iter().filter(|module| !module.enabled).count();
    format!("{on} enabled · {off} disabled")
}

fn harmonia_update_pressure_label(status: &crate::UpdatesStatus) -> String {
    let (ready, enabled) = harmonia_module_readiness(&status.modules);
    let missing = enabled.saturating_sub(ready);
    if status.check_missing_signal == "not-checked" {
        return "Check not run".to_string();
    }
    if !status.check_ok {
        if missing > 0 {
            return format!("Drift · {missing} missing");
        }
        if status.check_changed {
            return "Drift detected".to_string();
        }
        return "Check found work".to_string();
    }
    if !status.suite_ok {
        if status.suite_changed {
            return "Apply needed".to_string();
        }
        return "Suite stale".to_string();
    }
    if missing > 0 {
        return format!("{missing} module(s) missing");
    }
    "No pressure".to_string()
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

fn harmonia_update_availability_pane(status: &crate::UpdatesStatus) -> Markup {
    let tiles = harmonia_update_tiles(status);
    html! {
        article class=(format!("harmonia-update-pane harmonia-update-pane--{}", if tiles.is_empty() { "zero" } else { "available" })) data-harmonia-update-pane="true" data-update-count=(status.pending_updates) {
            @if tiles.is_empty() {
                div class="harmonia-zero-updates" data-zero-updates="true" {
                    strong { "Zero updates available" }
                    span { (harmonia_update_pressure_label(status)) }
                }
            } @else {
                div class="harmonia-pane-chrome" {
                    strong { (harmonia_pending_label(status.pending_updates)) " available" }
                    span { (harmonia_update_pressure_label(status)) }
                }
                div class="harmonia-update-tiles" data-harmonia-update-tiles="true" {
                    @for (kind, label, value, detail) in tiles {
                        article class=(format!("harmonia-update-tile harmonia-update-tile--{}", kind)) data-update-kind=(kind) {
                            strong { (label) }
                            b { (value) }
                            span { (detail) }
                        }
                    }
                }
            }
        }
    }
}

fn harmonia_update_tiles(
    status: &crate::UpdatesStatus,
) -> Vec<(&'static str, &'static str, String, String)> {
    let (ready, enabled) = harmonia_module_readiness(&status.modules);
    let missing = enabled.saturating_sub(ready);
    let mut tiles = Vec::new();

    if status.pending_updates == 0 && missing == 0 && status.check_ok && status.suite_ok {
        return tiles;
    }

    if missing > 0 {
        tiles.push((
            "modules",
            "Modules",
            format!("{missing} missing"),
            format!("{ready}/{enabled} enabled modules ready"),
        ));
    }

    if status.check_missing_signal == "not-checked" {
        tiles.push((
            "check",
            "State check",
            "Not run".to_string(),
            "Use Check state for a fresh readback".to_string(),
        ));
    } else if !status.check_ok || status.check_changed {
        tiles.push((
            "check",
            "State check",
            harmonia_check_status_label(status),
            harmonia_missing_signal_label(&status.check_missing_signal),
        ));
    }

    if !status.suite_ok || status.suite_changed {
        tiles.push((
            "suite",
            "System suite",
            harmonia_suite_status_label(status),
            harmonia_missing_signal_label(&status.first_missing_signal),
        ));
    }

    if status.state == "available" {
        tiles.push((
            "arcadia",
            "Arcadia GUI",
            status
                .available_version
                .clone()
                .unwrap_or_else(|| "Available".to_string()),
            format!("Current {}", status.current_version),
        ));
    }

    if tiles.is_empty() && status.pending_updates > 0 {
        tiles.push((
            "harmonia",
            "Harmonia",
            harmonia_pending_label(status.pending_updates),
            format!("Last run {}", status.last_update_run),
        ));
    }

    tiles
}

fn harmonia_module_row(module: &crate::HarmoniaModuleStatus) -> Markup {
    html! {
        article class=(format!("harmonia-module harmonia-module--{}", module.state)) data-harmonia-module=(module.id) data-module-enabled=(module.enabled) {
            div {
                strong { (module.label) }
                span { (module.id) }
            }
            b class=(format!("system-status system-status--{}", if module.enabled && module.present { "available" } else if module.enabled { "error" } else { "disabled" })) {
                (if module.enabled { if module.present { "Enabled" } else { "Missing" } } else { "Disabled" })
            }
            button class="btn btn--secondary" type="button" data-harmonia-module-toggle=(module.id) data-enabled=(module.enabled) {
                (if module.enabled { "Disable" } else { "Enable" })
            }
        }
    }
}
