fn updates_view(status: &ConsoleStatus) -> Markup {
    let pane = crate::api_updates_pane_state(status);
    let pane_json = serde_json::to_string(&pane)
        .unwrap_or_else(|_| "{}".to_string())
        .replace('<', "\\u003c");
    view_shell(
        "updates",
        "",
        "",
        "",
        html! {
        div class="updates-pane harmonia-panel harmonia-panel--update-center" data-updates-pane-family="updatesPane" data-harmonia-updates="true" data-bind-class="updatesPane.stateClass" {
            script id="updates-pane-state" type="application/json" { (PreEscaped(pane_json)) }
            header class="updates-header" {
                span class="updates-state-pill" data-state=(pane.state_class) data-bind-class="updatesPane.stateClass" role="status" data-bind="updatesPane.stateLabel" { (pane.state_label) }
                div class="updates-actions" data-harmonia-update-controls="true" {
                    (action_button(ButtonVariant::Secondary, "Check", "check-updates", "/api/actions/check-updates"))
                    (action_button(ButtonVariant::Primary, "Update now", "update-gui", "/api/actions/update-gui"))
                    button class="btn btn--secondary" type="button" data-harmonia-ledger-open="true" { "History" }
                }
            }

            section class="updates-automatic" aria-label="Automatic updates" {
                label class="updates-automatic-switch" data-harmonia-automatic-updates-switch="true" {
                    input type="checkbox" data-harmonia-automatic-updates-toggle="true" aria-label="Automatic updates" aria-checked="mixed" disabled;
                    span class="pin-toggle-track" aria-hidden="true" { span class="pin-toggle-thumb" {} }
                    span class="updates-automatic-copy" {
                        strong { "Automatic updates" }
                        span class="updates-automatic-status" data-harmonia-automatic-updates-status="true" aria-live="polite" { "Loading service state…" }
                    }
                }
                button class="btn btn--ghost updates-details-link" type="button" data-updates-details="automatic" { "Details" }
            }

            main class="updates-board" data-harmonia-default-grid="true" {
                section class="updates-module-pane" aria-label="Your update modules" {
                    header class="updates-inventory-heading" {
                        div {
                            h2 { "Your modules" }
                            span class="updates-inventory-count" data-bind="updatesPane.moduleCount" { (pane.module_count) }
                        }
                        span class="updates-inventory-summary" data-bind="updatesPane.moduleLine" { (pane.module_line) }
                    }
                    article class="updates-core" data-updates-core="true" data-bind-show="updatesPane.coreAvailable" hidden[!pane.core_available] {
                        header class="updates-core-head" {
                            div { strong { "Core" } span { "Kept together" } }
                            button class="btn btn--ghost updates-details-link" type="button" data-updates-details="core" { "Details ↗" }
                        }
                        div class="updates-core-grid" data-bind-each="updatesPane.coreModules" data-bind-replace="true" {
                            template { (updates_core_member(None)) }
                            @for module in &pane.core_modules { (updates_core_member(Some(module))) }
                        }
                    }
                    div class="updates-module-grid" data-harmonia-module-grid="true" data-bind-each="updatesPane.modules" data-bind-replace="true" {
                        template { (updates_module_card(None)) }
                        @for module in &pane.modules { (updates_module_card(Some(module))) }
                    }
                    p class="updates-inventory-empty" data-bind="updatesPane.modulesEmptyLabel" data-bind-show="updatesPane.modulesEmptyLabel" hidden[pane.modules_empty_label.is_empty()] { (pane.modules_empty_label) }
                    }

                aside class="updates-attention" data-state=(pane.state_class) data-bind-class="updatesPane.stateClass" aria-live="polite" {
                    span class="updates-attention-kicker" data-bind="updatesPane.checkLabel" { (pane.check_label) }
                    h2 data-bind="updatesPane.attentionTitle" { (pane.attention_title) }
                    p data-bind="updatesPane.attentionCopy" { (pane.attention_copy) }
                    p class="updates-attention-detail" data-bind="updatesPane.checkDetail" { (pane.check_detail) }
                    button class="btn btn--ghost updates-details-link" type="button" data-updates-details="evidence" { "Evidence details ↗" }
                }
            }

            footer class="updates-footer" {
                span { "— means the source did not provide that evidence." }
                button class="btn btn--ghost updates-details-link" type="button" data-updates-details="evidence" { "Evidence details" }
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

fn updates_core_member(module: Option<&crate::ApiUpdatesModuleState>) -> Markup {
    let id = module.map(|module| module.id.as_str()).unwrap_or("");
    let label = module.map(|module| module.label.as_str()).unwrap_or("");
    let state_label = module.map(|module| module.state_label.as_str()).unwrap_or("Unknown");
    let state_class = module.map(|module| module.state_class).unwrap_or("unknown");
    let current_identity = module.map(|module| module.current_identity.as_str()).unwrap_or("—");
    let target_identity = module.map(|module| module.target_identity.as_str()).unwrap_or("—");
    let proof_date = module.map(|module| module.proof_date.as_str()).unwrap_or("—");
    let release_date = module.map(|module| module.release_date.as_str()).unwrap_or("—");
    html! {
        article class="updates-core-member" data-harmonia-module=(id) data-bind-attr-id="id" data-state=(state_class) data-bind-class="stateClass" {
            header class="updates-core-member-head" {
                strong data-bind="label" { (label) }
            }
            div class="updates-compare" {
                div { span { "Now" } strong data-bind="currentIdentity" { (current_identity) } }
                span class="updates-compare-arrow" aria-hidden="true" { "→" }
                div { span { "Next" } strong data-bind="targetIdentity" { (target_identity) } }
            }
            div class="updates-date-pair" {
                span { "Proof " b data-bind="proofDate" { (proof_date) } }
                span { "Release " b data-bind="releaseDate" { (release_date) } }
            }
            button class="btn btn--ghost updates-module-details" type="button" data-updates-module-details="true" aria-label=(format!("Details for {}", label)) { "Details" }
            span class="updates-sr-only" data-bind="stateLabel" { (state_label) }
        }
    }
}

fn updates_module_card(module: Option<&crate::ApiUpdatesModuleState>) -> Markup {
    let id = module.map(|module| module.id.as_str()).unwrap_or("");
    let label = module.map(|module| module.label.as_str()).unwrap_or("");
    let enabled = module.is_some_and(|module| module.enabled);
    let state_label = module.map(|module| module.state_label.as_str()).unwrap_or("Unknown");
    let state_class = module.map(|module| module.state_class).unwrap_or("unknown");
    let membership_label = module.map(|module| module.membership_label.as_str()).unwrap_or("Membership unknown");
    let current_identity = module.map(|module| module.current_identity.as_str()).unwrap_or("—");
    let target_identity = module.map(|module| module.target_identity.as_str()).unwrap_or("—");
    let proof_date = module.map(|module| module.proof_date.as_str()).unwrap_or("—");
    let release_date = module.map(|module| module.release_date.as_str()).unwrap_or("—");
    let update_allowed = module.is_some_and(|module| module.update_allowed);
    let update_label = module.map(|module| module.update_label).unwrap_or("Update");
    let update_message = module.map(|module| module.update_message).unwrap_or("");
    html! {
        article class="updates-module" data-harmonia-module=(id) data-bind-attr-id="id" data-state=(state_class) data-bind-class="stateClass" {
            header class="updates-module-head" {
                span class="updates-module-mark" aria-hidden="true" {
                    svg viewBox="0 0 20 20" focusable="false" { path d="M3 6 10 2l7 4v8l-7 4-7-4z M3 6l7 4 7-4 M10 10v8" {} }
                }
                div class="updates-module-title" {
                    strong data-bind="label" { (label) }
                    span class="updates-module-membership" data-bind="membershipLabel" { (membership_label) }
                }
                label class="updates-module-switch" data-harmonia-module-switch-row=(id) {
                    input type="checkbox" value=(id) checked[enabled] data-harmonia-module-switch="" data-bind-value="id" data-bind-checked="enabled" data-bind-aria-label="label" aria-label=(format!("Enable {} module", label));
                    span class="pin-toggle-track" aria-hidden="true" { span class="pin-toggle-thumb" {} }
                }
            }
            div class="updates-compare" aria-label="Current and target identity" {
                div { span { "Now" } strong data-bind="currentIdentity" { (current_identity) } }
                span class="updates-compare-arrow" aria-hidden="true" { "→" }
                div { span { "Next" } strong data-bind="targetIdentity" { (target_identity) } }
            }
            div class="updates-date-pair" {
                span { "Proof " b data-bind="proofDate" { (proof_date) } }
                span { "Release " b data-bind="releaseDate" { (release_date) } }
            }
            footer class="updates-module-foot" {
                div class="updates-module-actions" {
                    button class="btn btn--ghost updates-module-details" type="button" data-updates-module-details="true" aria-label=(format!("Details for {}", label)) { "Details" }
                    button class="btn btn--secondary updates-module-update" type="button" data-harmonia-module-update=(id) data-update-endpoint="/api/actions/update-module" data-bind="updateLabel" data-bind-show="updateAllowed" hidden[!update_allowed] aria-label=(format!("Update {} module", label)) { (update_label) }
                }
                span class="updates-module-update-message" data-harmonia-module-update-message=(id) data-bind="updateMessage" data-bind-show="updateMessage" hidden[update_message.is_empty()] { (update_message) }
            }
            span class="updates-sr-only" data-bind="stateLabel" { (state_label) }
        }
    }
}
