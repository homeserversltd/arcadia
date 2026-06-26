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
    let unavailable = enabled.saturating_sub(ready);
    if status.check_missing_signal == "not-checked" {
        return "Not checked".to_string();
    }
    if !status.check_ok || status.check_changed {
        return "Updates available".to_string();
    }
    if !status.suite_ok || status.suite_changed {
        return "Update needed".to_string();
    }
    if unavailable > 0 {
        return format!("{unavailable} module(s) unavailable");
    }
    "Current".to_string()
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
                    strong { (harmonia_update_modules_heading(tiles.len())) }
                    span { (harmonia_update_pressure_label(status)) }
                }
                div class="harmonia-update-tiles" data-harmonia-update-tiles="true" {
                    @for (kind, label, value, detail) in tiles {
                        article class="harmonia-update-tile harmonia-update-tile--module" data-update-kind=(kind) {
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

fn harmonia_update_modules_heading(count: usize) -> String {
    if count == 1 {
        "1 module needs update".to_string()
    } else {
        format!("{count} modules need update")
    }
}

fn harmonia_update_tiles(status: &crate::UpdatesStatus) -> Vec<(&str, &str, String, String)> {
    let mut tiles = Vec::new();

    for module in status
        .modules
        .iter()
        .filter(|module| module.enabled && !module.present)
    {
        tiles.push((
            module.id.as_str(),
            module.label.as_str(),
            "Update needed".to_string(),
            "Press Sync to update this module".to_string(),
        ));
    }

    if tiles.is_empty() && (!status.check_ok || status.check_changed || status.pending_updates > 0) {
        tiles.push((
            "enabled-modules",
            "Enabled modules",
            harmonia_pending_label(status.pending_updates.max(1)),
            "Press Sync to update enabled modules".to_string(),
        ));
    }

    tiles
}

fn harmonia_module_row(module: &crate::HarmoniaModuleStatus) -> Markup {
    let status_tone = if module.enabled && module.present {
        "available"
    } else if module.enabled {
        "error"
    } else {
        "disabled"
    };
    let status_label = if module.enabled {
        if module.present { "Enabled" } else { "Update needed" }
    } else {
        "Disabled"
    };
    html! {
        article class=(format!("harmonia-module harmonia-module--{}", module.state)) data-harmonia-module=(module.id) data-module-enabled=(module.enabled) {
            label class="harmonia-module-switch" data-harmonia-module-switch-row=(module.id) {
                input type="checkbox" checked[module.enabled] data-harmonia-module-switch=(module.id) data-enabled=(module.enabled) aria-label=(format!("{} module enabled", module.label));
                span class="pin-toggle-track" aria-hidden="true" {
                    span class="pin-toggle-thumb" {}
                }
                span class="harmonia-module-copy" {
                    strong { (module.label) }
                    span { (module.id) }
                }
            }
            b class=(format!("system-status system-status--{}", status_tone)) { (status_label) }
        }
    }
}
