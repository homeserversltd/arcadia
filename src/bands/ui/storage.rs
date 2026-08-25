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
                            strong { span data-bind="storagePane.hero.free" { (status.storage.free) } " free" }
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

                template id="storage-games-modal-template" { div class="storage-detail-modal" data-storage-presenter="true" { nav class="storage-breadcrumb" { span { "Storage / Games" } } div class="storage-detail-content" { div class="network-detail-row" { span { em { "Summary" } strong data-bind="storage.categories.games.detail" {} } } div class="storage-detail-table" data-bind-each="storage.gameFolders" { template { div class="storage-detail-table-row" { span data-bind="displayName" {} span data-bind="size" {} span data-bind="fileCount" {} span data-bind="syncedEntries" {} button type="button" data-storage-game-open data-bind-value="platform" { "Open" } button type="button" data-storage-copy-path data-bind-copy-value="path" { "Local" } button type="button" data-storage-copy-path data-bind-copy-value="windowsUNC" { "Windows" } button type="button" data-storage-copy-path data-bind-copy-value="windowsUNCByIp" { "Windows IP" } button type="button" data-storage-copy-path data-bind-copy-value="smbUrl" { "SMB" } button type="button" data-storage-copy-path data-bind-copy-value="smbUrlByIp" { "SMB IP" } } } } } } }
                template id="storage-game-detail-modal-template" { div class="storage-detail-modal" data-storage-presenter="true" { nav class="storage-breadcrumb" { button type="button" class="btn btn--secondary" data-storage-games-back { "Back" } span { "Storage / Games / Detail" } } div class="storage-detail-content" { div class="network-detail-row" { span { em { "Path" } strong data-bind="storage.gameFolders.0.path" {} } } div class="inline-actions" { button type="button" data-storage-copy-path data-bind-copy-value="storage.gameFolders.0.path" { "Local" } button type="button" data-storage-copy-path data-bind-copy-value="storage.gameFolders.0.windowsUNC" { "Windows" } button type="button" data-storage-copy-path data-bind-copy-value="storage.gameFolders.0.windowsUNCByIp" { "Windows IP" } button type="button" data-storage-copy-path data-bind-copy-value="storage.gameFolders.0.smbUrl" { "SMB" } button type="button" data-storage-copy-path data-bind-copy-value="storage.gameFolders.0.smbUrlByIp" { "SMB IP" } } div class="network-detail-row" { span { em { "Files" } strong data-bind="storage.gameFolders.0.fileCount" {} } span { em { "Size" } strong data-bind="storage.gameFolders.0.size" {} } } div class="storage-detail-table" data-bind-each="storage.gameFolders.0.largestFiles" { template { div class="storage-detail-table-row" { span data-bind="name" {} span data-bind="size" {} span data-bind="path" {} button type="button" data-storage-copy-path data-bind-copy-value="path" { "Copy" } } } } } } }
                template id="storage-artwork-modal-template" { div class="storage-detail-modal" data-storage-presenter="true" { nav class="storage-breadcrumb" { span { "Storage / Artwork" } } div class="storage-detail-table" data-bind-each="storage.artworkStores" { template { div class="storage-detail-table-row" { span data-bind="displayName" {} span data-bind="size" {} span data-bind="fileCount" {} span data-bind="state" {} button type="button" data-storage-copy-path data-bind-copy-value="path" { "Copy path" } } } } } }
                template id="storage-ai-models-modal-template" { div class="storage-detail-modal" data-storage-presenter="true" { nav class="storage-breadcrumb" { span { "Storage / Local AI" } } div class="storage-detail-table" data-bind-each="storage.aiModelFiles" { template { div class="storage-detail-table-row" { span data-bind="name" {} span data-bind="size" {} span data-bind="loaded" {} span data-bind="path" {} button type="button" data-storage-copy-path data-bind-copy-value="path" { "Copy path" } } } } div class="storage-detail-table" data-bind-each="storage.registry.categories.aiModels.roots" { template { div class="storage-detail-table-row" { span data-bind="displayName" {} span data-bind="purpose" {} span data-bind="path" {} button type="button" data-storage-copy-path data-bind-copy-value="path" { "Copy path" } } } } } }
                @for (id, label, category, roots) in [("updates", "Updates", "updates", "updates"), ("logs", "Logs", "logs", "logs"), ("temporary", "Temporary", "temporary", "temporary"), ("system", "System", "system", "system")] { template id=(format!("storage-{}-modal-template", id)) { div class="storage-detail-modal" data-storage-presenter="true" { nav class="storage-breadcrumb" { span { "Storage / " (label) } } div class="network-detail-row" { span { em { "Size" } strong data-bind=(format!("storage.categories.{}.size", category)) {} } span { em { "Detail" } strong data-bind=(format!("storage.categories.{}.detail", category)) {} } } div class="storage-detail-table" data-bind-each=(format!("storage.registry.categories.{}.roots", roots)) { template { div class="storage-detail-table-row" { span data-bind="displayName" {} span data-bind="purpose" {} span data-bind="path" {} button type="button" data-storage-copy-path data-bind-copy-value="path" { "Copy" } } } } } } }
                template id="storage-category-other-modal-template" { div class="storage-detail-modal" data-storage-presenter="true" { nav class="storage-breadcrumb" { span { "Storage / Other" } } div class="network-detail-row" { span { em { "Size" } strong data-bind="storage.categories.other.size" {} } span { em { "Detail" } strong data-bind="storage.categories.other.detail" {} } } } }
                template id="storage-category-free-modal-template" { div class="storage-detail-modal" data-storage-presenter="true" { nav class="storage-breadcrumb" { span { "Storage / Free" } } div class="network-detail-row" { span { em { "Available" } strong data-bind="storage.free" {} } } } }
                template id="storage-cleanup-modal-template" { div class="storage-detail-modal" data-storage-presenter="true" { nav class="storage-breadcrumb" { span { "Storage / Cleanup" } } @for (label, value, action, endpoint) in [("Artwork cache", "artworkBytesClearable", "clear-artwork-cache", "/api/storage/cleanup/artwork"), ("Temporary files", "temporaryBytesClearable", "clean-temporary-files", "/api/storage/cleanup/temporary"), ("Old updates", "oldUpdateBytesClearable", "clear-old-updates", "/api/storage/cleanup/old-updates"), ("Logs", "logsBytesClearable", "prune-logs", "/api/storage/cleanup/logs"), ("Partial downloads", "partialDownloadsBytesClearable", "clear-partial-ai-downloads", "/api/storage/cleanup/partial-ai-downloads")] { div class="storage-detail-table-row" { span { (label) } span data-bind=(format!("storage.cleanup.{}", value)) data-bind-zero-dash="true" {} button type="button" data-bind-enabled=(format!("storage.cleanup.{}", value)) data-storage-cleanup-action=(action) data-storage-cleanup-endpoint=(endpoint) { "Clear" } } } } }
                template id="storage-locations-modal-template" { div class="storage-detail-modal" data-storage-presenter="true" { nav class="storage-breadcrumb" { span { "Storage / Managed Locations" } } @for (name, category) in [("Games", "games"), ("Artwork", "artwork"), ("AI Models", "aiModels"), ("Updates", "updates"), ("Logs", "logs"), ("Temporary", "temporary"), ("System", "system")] { div class="storage-location-group" { strong { (name) } div class="storage-detail-table" data-bind-each=(format!("storage.registry.categories.{}.roots", category)) { template { div class="storage-detail-table-row" { span data-bind="displayName" {} span data-bind="path" {} button type="button" data-storage-copy-path data-bind-copy-value="path" { "Copy" } } } } } } } }
                template id="storage-diagnostics-modal-template" { div class="storage-detail-modal" data-storage-presenter="true" { nav class="storage-breadcrumb" { span { "Storage / Diagnostics" } } @for (label, path) in [("Mount point", "mountPoint"), ("Filesystem", "filesystem"), ("Scan duration", "scanDurationMs"), ("Scanner", "scannerVersion"), ("Last scan", "lastScanTimestamp")] { div class="network-detail-row" { span { em { (label) } strong data-bind=(format!("storage.diagnostics.{}", path)) {} } } } @for (label, path) in [("Warnings", "warnings"), ("Overlaps", "overlapWarnings"), ("Category errors", "categoryScanErrors"), ("Missing dirs", "missingDirs"), ("Permission errors", "permissionErrors")] { div class="storage-detail-table" { strong { (label) } div data-bind-each=(format!("storage.diagnostics.{}", path)) { template { div class="storage-detail-table-row" { span data-bind="." {} } } } } } } }
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
        span class="storage-segment storage-segment--free" style=(format!("width: {}%", 100u8.saturating_sub(status.storage.percent_used))) data-bind-style-var="--storage-seg-pct:storagePane.capacity.segments.free.width" title=(format!("Free {}", status.storage.free)) {}
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

