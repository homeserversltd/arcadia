fn updates_view(status: &ConsoleStatus) -> Markup {
    view_shell(
        "updates",
        "",
        "",
        "",
        html! {
            div class="updates-pane harmonia-panel harmonia-panel--update-center" data-updates-pane-family="updatesPane" data-harmonia-updates="true" data-bind-class="updatesPane.stateClass" {
                div class="updates-status-board" data-updates-status-board="true" {
                    article class="updates-state-card updates-state-card--hero" {
                        span class="updates-kicker" { "Harmonia state" }
                        strong data-bind="updatesPane.stateLabel" { (harmonia_update_pressure_label(&status.updates)) }
                        div class="updates-state-meta" {
                            span { "Last ran " b data-bind="updatesPane.lastRan" { (status.updates.last_update_run) } }
                            span { "Updates " b data-bind="updatesPane.pendingUpdates" { (status.updates.pending_updates) } }
                            span { "Modules " b data-bind="updatesPane.readinessRatio" { (harmonia_module_readiness_line(&status.updates.modules)) } }
                        }
                    }
                    div class="harmonia-command-rail updates-command-rail" data-harmonia-update-controls="true" {
                        (action_button(ButtonVariant::Secondary, "Check state", "check-updates", "/api/actions/check-updates"))
                        (action_button(ButtonVariant::Primary, "Sync", "update-gui", "/api/actions/update-gui"))
                        button class="btn btn--secondary" type="button" data-harmonia-ledger-open="true" { "Ledger" }
                    }
                }

                div class="updates-board" data-harmonia-default-grid="true" {
                    article class="harmonia-module-pane updates-module-pane" data-harmonia-module-pane="true" {
                        div class="harmonia-pane-chrome updates-pane-chrome" {
                            strong { "Modules" }
                            span data-bind="updatesPane.moduleLine" { (harmonia_module_grid_line(&status.updates.modules)) }
                        }
                        div class="harmonia-module-grid updates-module-grid" data-harmonia-module-grid="true" data-bind-each="updatesPane.modules" data-bind-replace="true" {
                            template {
                                article class="updates-module" data-harmonia-module="" data-bind-class="stateClass" data-bind-attr-id="id" data-module-enabled="" {
                                    label class="updates-module-switch" data-harmonia-module-switch-row="" {
                                        input type="checkbox" data-harmonia-module-switch="" data-bind-checked="enabled" data-bind-value="id" aria-label="Harmonia module enabled";
                                        span class="pin-toggle-track" aria-hidden="true" { span class="pin-toggle-thumb" {} }
                                        span class="updates-module-copy" {
                                            strong data-bind="label" {}
                                            span data-bind="description" {}
                                            span data-bind="id" {}
                                        }
                                    }
                                    span class="updates-module-version" { em { "Version" } strong data-bind="version" {} }
                                    div class="updates-module-action" data-bind-show="updateAllowed" {
                                        button class="btn btn--secondary updates-module-update" type="button" data-harmonia-module-update="" data-bind="updateLabel" aria-label="Update Harmonia module" {}
                                        span class="updates-module-update-message" data-bind="updateMessage" {}
                                    }
                                    b class="system-status" data-bind-class="stateClass" data-bind="stateLabel" {}
                                }
                            }
                            @for module in &status.updates.modules {
                                @if module.pinned_module_membership.is_none() || module.pinned_module_membership.as_deref() == Some("unpinned") {
                                    (harmonia_module_row(module, &status.updates.current_version))
                                }
                            }
                        }
                        div class="updates-pinned-grid" data-bind-each="updatesPane.pinned" data-bind-replace="true" {
                            template {
                                article class="updates-pinned-module" data-harmonia-pinned-update="true" {
                                    div class="updates-pinned-copy" {
                                        strong data-bind="label" {}
                                        div class="updates-pinned-members" data-bind-each="members" {
                                            template { span data-bind="." {} }
                                        }
                                        span class="updates-pinned-message" data-harmonia-suite-update-message="true" data-bind="message" {}
                                    }
                                    button class="btn btn--primary updates-pinned-update" type="button" data-harmonia-suite-update="true" data-endpoint="/api/actions/update-gui" data-bind="buttonLabel" {}
                                }
                            }
                            (harmonia_pinned_group(status))
                        }
                    }
                    (harmonia_update_availability_pane(&status.updates))
                }
                details class="collapsible-log harmonia-receipts updates-receipts" {
                    summary { "Readbacks" }
                    div class="system-field-grid" {
                        (updates_system_field_bound("Suite", "updatesPane.receipts.suite", &status.updates.latest_receipt))
                        (updates_system_field_bound("Check", "updatesPane.receipts.check", &status.updates.latest_check_receipt))
                        (updates_system_field_bound("Module root", "updatesPane.receipts.moduleRoot", &status.updates.module_root))
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

fn harmonia_module_readiness_line(modules: &[crate::HarmoniaModuleStatus]) -> String {
    let (ready, enabled) = harmonia_module_readiness(modules);
    format!("{ready}/{enabled}")
}

fn harmonia_module_grid_line(modules: &[crate::HarmoniaModuleStatus]) -> String {
    let (ready, enabled) = harmonia_module_readiness(modules);
    let waiting = enabled.saturating_sub(ready);
    format!("{ready} ready · {waiting} need update")
}

fn harmonia_update_pressure_label(status: &crate::UpdatesStatus) -> String {
    let (ready, enabled) = harmonia_module_readiness(&status.modules);
    let unavailable = enabled.saturating_sub(ready);
    if status.check_missing_signal == "not-checked" {
        return "Check needed".to_string();
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
        article class=(format!("harmonia-update-pane updates-available-pane harmonia-update-pane--{}", if tiles.is_empty() { "zero" } else { "available" })) data-harmonia-update-pane="true" data-update-count=(status.pending_updates) {
            div class="harmonia-pane-chrome updates-pane-chrome" {
                strong { "Available" }
                span data-bind="updatesPane.pendingUpdates" { (harmonia_pending_label(status.pending_updates)) }
            }
            @if tiles.is_empty() {
                div class="harmonia-zero-updates" data-zero-updates="true" data-harmonia-last-run="true" {
                    strong { "Zero updates available" }
                    span { (harmonia_update_pressure_label(status)) }
                    div class="updates-last-run" {
                        b { "Last run" }
                        span { "Loading the latest module results…" }
                    }
                }
            } @else {
                div class="updates-update-heading" { (harmonia_update_modules_heading(tiles.len())) }
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

    if tiles.is_empty() && (!status.check_ok || status.check_changed || status.pending_updates > 0)
    {
        tiles.push((
            "enabled-modules",
            "Enabled modules",
            harmonia_pending_label(status.pending_updates.max(1)),
            "Press Sync to update enabled modules".to_string(),
        ));
    }

    tiles
}

fn harmonia_pinned_group(status: &ConsoleStatus) -> Markup {
    let members = status
        .updates
        .modules
        .iter()
        .filter(|module| {
            module
                .pinned_module_membership
                .as_deref()
                .is_some_and(|membership| membership != "unpinned")
        })
        .map(|module| module.label.clone())
        .collect::<Vec<_>>();
    if members.is_empty() {
        return html! {};
    }
    html! {
        article class="updates-pinned-module" data-harmonia-pinned-update="true" {
            div class="updates-pinned-copy" {
                strong { "Pinned modules" }
                div class="updates-pinned-members" {
                    @for member in members { span { (member) } }
                }
                span class="updates-pinned-message" data-harmonia-suite-update-message="true" { "Updates this group with the whole-suite Sync action." }
            }
            button class="btn btn--primary updates-pinned-update" type="button" data-harmonia-suite-update="true" data-endpoint="/api/actions/update-gui" { "Update pinned modules" }
        }
    }
}

fn harmonia_module_row(module: &crate::HarmoniaModuleStatus, current_version: &str) -> Markup {
    let status_tone = if module.enabled && module.present {
        "available"
    } else if module.enabled {
        "error"
    } else {
        "disabled"
    };
    let status_label = if module.enabled {
        if module.present {
            "Enabled"
        } else {
            "Update needed"
        }
    } else {
        "Disabled"
    };
    let version = if module.present {
        current_version
    } else {
        "Pending"
    };
    html! {
        article class="updates-module" data-state=(status_tone) data-harmonia-module=(module.id) data-module-enabled=(module.enabled) data-bind-class="updatesPane.modules.stateClass" {
            label class="updates-module-switch" data-harmonia-module-switch-row=(module.id) {
                input type="checkbox" checked[module.enabled] data-harmonia-module-switch=(module.id) data-enabled=(module.enabled) aria-label=(format!("{} module enabled", module.label));
                span class="pin-toggle-track" aria-hidden="true" { span class="pin-toggle-thumb" {} }
                span class="updates-module-copy" {
                    strong { (module.label) }
                    @if !module.description.is_empty() {
                        span class="updates-module-description" { (&module.description) }
                    }
                    span { (module.id) }
                }
            }
            span class="updates-module-version" { em { "Version" } strong { (version) } }
            @if module.pinned_module_membership.as_deref() == Some("unpinned") {
                div class="updates-module-action" {
                    button class="btn btn--secondary updates-module-update" type="button" data-harmonia-module-update=(module.id) data-update-endpoint="/api/actions/update-module" aria-label=(format!("Update {} module", module.label)) { "Update module" }
                    span class="updates-module-update-message" data-harmonia-module-update-message=(module.id) { "Ready for an independent update." }
                }
            }
            b class=(format!("system-status system-status--{}", status_tone)) { (status_label) }
        }
    }
}

fn updates_system_field_bound(label: &str, bind: &str, value: &str) -> Markup {
    html! {
        div class="system-field" {
            span { (label) }
            strong data-bind=(bind) { (value) }
        }
    }
}
