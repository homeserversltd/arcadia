fn storage_view(status: &ConsoleStatus) -> Markup {
    let cleanup_total = status
        .storage
        .cleanup
        .artwork_bytes_clearable
        .saturating_add(status.storage.cleanup.temporary_bytes_clearable)
        .saturating_add(status.storage.cleanup.partial_downloads_bytes_clearable)
        .saturating_add(status.storage.cleanup.old_update_bytes_clearable)
        .saturating_add(status.storage.cleanup.logs_bytes_clearable);
    let diagnostics_count = storage_diagnostics_count(status);
    let classified_bytes = status
        .storage
        .games
        .bytes
        .saturating_add(status.storage.artwork.bytes)
        .saturating_add(status.storage.ai_models.bytes)
        .saturating_add(status.storage.categories.updates.bytes)
        .saturating_add(status.storage.categories.logs.bytes)
        .saturating_add(status.storage.categories.temporary.bytes)
        .saturating_add(status.storage.categories.system.bytes);
    let accounted_bytes = classified_bytes.saturating_add(status.storage.other.bytes);
    let mismatch_copy = if diagnostics_count > 0 {
        storage_mismatch_copy(status, classified_bytes, accounted_bytes)
    } else if status.storage.percent_used >= status.storage.thresholds.low_percent {
        format!(
            "Filesystem reports {} used and {} free. Review cleanup before downloads or sync.",
            status.storage.used, status.storage.free
        )
    } else {
        format!(
            "Filesystem reports {} used. Category scan accounts for {} classified plus {} other.",
            status.storage.used,
            human_or_zero(classified_bytes),
            status.storage.other.size
        )
    };
    view_shell(
        "storage",
        "",
        "",
        "",
        html! {
            section class="storage-appliance storage-appliance--one-pane" aria-label="Storage overview" {
                article class=(if diagnostics_count > 0 { "storage-summary storage-dashboard storage-dashboard--warning" } else { "storage-summary storage-dashboard" }) {
                    div class="storage-command-center" {
                        div class="storage-health-block" {
                            span class=(if diagnostics_count > 0 { "status-pill partial" } else { "status-pill up" }) {
                                @if diagnostics_count > 0 { "Mismatch detected" } @else { "Storage " (status.storage.health) }
                            }
                            strong { (status.storage.free) " free" }
                            em { (status.storage.percent) " used · scanned " (human_scan_time(&status.storage.scanned_at)) }
                        }
                        div class="storage-actions" aria-label="Storage actions" {
                            (action_button(ButtonVariant::Primary, "Rescan", "storage-rescan", "/api/storage/rescan-summary"))
                            @if diagnostics_count > 0 {
                                button class="btn btn--secondary" type="button" data-storage-modal="diagnostics" { "Review mismatch" }
                            } @else if cleanup_total > 0 {
                                button class="btn btn--secondary" type="button" data-storage-modal="cleanup-review" { "Review cleanup" }
                            }
                            button class="btn btn--secondary" type="button" data-storage-modal="locations" { "Managed locations" }
                        }
                    }

                    div class="storage-hero-metrics" aria-label="Capacity summary" {
                        (storage_stat("Total", &status.storage.total))
                        (storage_stat("Used", &status.storage.used))
                        (storage_stat("Free", &status.storage.free))
                        (storage_stat("Scan", &human_scan_time(&status.storage.scanned_at)))
                    }

                    div class="storage-visual-panel" {
                        div class="storage-meter-head" {
                            strong { "Capacity" }
                            span { (status.storage.used) " used of " (status.storage.total) }
                        }
                        (storage_usage_bar(status))
                        div class="storage-legend storage-legend--inline" aria-label="Storage legend" {
                            (storage_legend_item("games", "Games", &status.storage.games.size))
                            (storage_legend_item("artwork", "Artwork", &status.storage.artwork.size))
                            (storage_legend_item("ai", "AI Models", &status.storage.ai_models.size))
                            (storage_legend_item("other", "Other", &status.storage.other.size))
                            (storage_legend_item("free", "Free", &status.storage.free))
                        }
                    }
                }

                article class=(if diagnostics_count > 0 { "storage-alert-panel storage-alert-panel--warning" } else { "storage-alert-panel" }) {
                    strong {
                        @if diagnostics_count > 0 { "Storage mismatch detected" }
                        @else if status.storage.percent_used >= status.storage.thresholds.low_percent { "Storage low" }
                        @else { "Storage accounting matches" }
                    }
                    span { (mismatch_copy) }
                    div class="storage-accounting-grid" aria-label="Storage accounting" {
                        (storage_accounting_fact("Filesystem used", &status.storage.used))
                        (storage_accounting_fact("Category scan", &human_or_zero(classified_bytes)))
                        (storage_accounting_fact("Other / unclassified", &status.storage.other.size))
                        (storage_accounting_fact("Scan time", &format!("{} ms", status.storage.diagnostics.scan_duration_ms)))
                    }
                }

                div class="storage-category-list storage-category-list--dashboard" aria-label="Storage consumption breakdown" {
                    (storage_category_row("🎮", "Games", &status.storage.games, "games", "games"))
                    (storage_category_row("🖼", "Artwork", &status.storage.artwork, "artwork", "artwork-detail"))
                    (storage_ai_category_row(&status.storage.ai_models))
                    (storage_category_row("⬇", "Updates", &status.storage.categories.updates, "updates", "updates-detail"))
                    (storage_category_row("≋", "Logs", &status.storage.categories.logs, "logs", "logs-detail"))
                    (storage_category_row("⌁", "Temporary Files", &status.storage.categories.temporary, "temporary", "cleanup-review"))
                    (storage_category_row("▣", "System", &status.storage.categories.system, "system", "system-detail"))
                    (storage_category_row("◇", "Other", &status.storage.other, "other", "category-other"))
                    (storage_free_row(status))
                }
            }
        },
    )
}

fn storage_diagnostics_count(status: &ConsoleStatus) -> usize {
    status.storage.diagnostics.missing_dirs.len()
        + status.storage.diagnostics.permission_errors.len()
        + status.storage.diagnostics.warnings.len()
        + status.storage.diagnostics.overlap_warnings.len()
        + status.storage.diagnostics.category_scan_errors.len()
}

fn storage_mismatch_copy(
    status: &ConsoleStatus,
    classified_bytes: u64,
    accounted_bytes: u64,
) -> String {
    format!(
        "Storage map conflict. Managed storage categories overlap or could not be fully scanned. Filesystem used {}. Managed categories classify {}; accounting total is {}. Rescan, then review mismatch if it remains.",
        status.storage.used,
        human_or_zero(classified_bytes),
        human_or_zero(accounted_bytes)
    )
}

fn storage_accounting_fact(label: &str, value: &str) -> Markup {
    html! { span class="storage-accounting-fact" { em { (label) } strong { (value) } } }
}

fn storage_legend_item(color: &str, label: &str, value: &str) -> Markup {
    html! { span class=(format!("storage-legend-item storage-legend-item--{}", color)) { em {} strong { (label) } small { (value) } } }
}

fn storage_category_row(
    icon: &str,
    label: &str,
    category: &StorageCategoryStatus,
    color: &str,
    modal: &str,
) -> Markup {
    html! {
        button class="storage-category-row storage-category-row--inline storage-category-row--action" type="button" data-storage-category=(color) data-storage-modal=(modal) aria-label=(format!("Open {} storage details", label)) {
            span class="storage-category-icon" { (icon) }
            div class="storage-category-main" {
                strong { (label) }
                small { (category.detail) }
            }
            b { (category.size) }
            em { (percent_label(category.percent_of_total, category.bytes)) }
            div class="storage-category-mini" aria-hidden="true" { span class=(format!("storage-segment--{}", color)) style=(format!("width: {}%", category.percent_of_total.max(if category.bytes == 0 { 0 } else { 1 }))) {} }
            span class=(format!("system-status system-status--{}", state_class(&category.state))) { (title_case_state_like(&category.state)) }
            span class="storage-category-chevron" aria-hidden="true" { "›" }
        }
    }
}

fn storage_ai_category_row(category: &AiModelStorageStatus) -> Markup {
    html! {
        button class="storage-category-row storage-category-row--inline storage-category-row--action" type="button" data-storage-category="ai" data-storage-modal="ai-models-detail" aria-label="Open Local AI storage details" {
            span class="storage-category-icon" { "◉" }
            div class="storage-category-main" {
                strong { "Local AI" }
                small { (category.detail) }
            }
            b { (category.size) }
            em { (percent_label(category.percent_of_total, category.bytes)) }
            div class="storage-category-mini" aria-hidden="true" { span class="storage-segment--ai" style=(format!("width: {}%", category.percent_of_total.max(if category.bytes == 0 { 0 } else { 1 }))) {} }
            span class="system-status system-status--available" { (category.meta) }
            span class="storage-category-chevron" aria-hidden="true" { "›" }
        }
    }
}

fn storage_free_row(status: &ConsoleStatus) -> Markup {
    let free_percent = 100u8.saturating_sub(status.storage.percent_used);
    html! {
        article class="storage-category-row storage-category-row--inline" data-storage-category="free" {
            span class="storage-category-icon" { "○" }
            div class="storage-category-main" {
                strong { "Free" }
                small { "Available capacity for games, artwork, updates, and Local AI models." }
            }
            b { (status.storage.free) }
            em { (percent_label(free_percent, status.storage.free_bytes)) }
            div class="storage-category-mini" aria-hidden="true" { span class="storage-segment--free" style=(format!("width: {}%", free_percent)) {} }
            span class="system-status system-status--available" { "Available" }
        }
    }
}

fn state_class(state: &str) -> &'static str {
    match state {
        "ok" => "available",
        "warning" => "starting",
        "unknown" => "unknown",
        _ => "unknown",
    }
}

fn human_scan_time(scanned_at: &str) -> String {
    if scanned_at.contains('T') {
        "just now".to_string()
    } else {
        scanned_at.to_string()
    }
}

fn storage_usage_bar(status: &ConsoleStatus) -> Markup {
    html! { div class="storage-bar" aria-label="Segmented storage usage" {
        (storage_segment("games", status.storage.games.percent_of_total, status.storage.games.bytes, &status.storage.games.size))
        (storage_segment("artwork", status.storage.artwork.percent_of_total, status.storage.artwork.bytes, &status.storage.artwork.size))
        (storage_segment("ai", status.storage.ai_models.percent_of_total, status.storage.ai_models.bytes, &status.storage.ai_models.size))
        (storage_segment("updates", status.storage.categories.updates.percent_of_total, status.storage.categories.updates.bytes, &status.storage.categories.updates.size))
        (storage_segment("logs", status.storage.categories.logs.percent_of_total, status.storage.categories.logs.bytes, &status.storage.categories.logs.size))
        (storage_segment("temporary", status.storage.categories.temporary.percent_of_total, status.storage.categories.temporary.bytes, &status.storage.categories.temporary.size))
        (storage_segment("system", status.storage.categories.system.percent_of_total, status.storage.categories.system.bytes, &status.storage.categories.system.size))
        span class="storage-segment storage-segment--free" style=(format!("width: {}%", 100u8.saturating_sub(status.storage.percent_used))) title=(format!("Free {}", status.storage.free)) {}
    } }
}

fn storage_segment(class: &str, percent: u8, bytes: u64, size: &str) -> Markup {
    let width = percent.max(if bytes > 0 { 1 } else { 0 });
    html! { span class=(format!("storage-segment storage-segment--{}", class)) style=(format!("width: {}%", width)) title=(size) {} }
}

fn human_or_zero(bytes: u64) -> String {
    if bytes == 0 {
        "0 B".to_string()
    } else {
        crate::human_size(bytes)
    }
}

