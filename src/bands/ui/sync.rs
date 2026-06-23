fn sync_view(status: &ConsoleStatus) -> Markup {
    let storage_blocked = status.storage.health == "Full";
    let storage_low = status.storage.percent_used >= 90;
    let pending_changes = status.library.unsynced_added
        + status.library.unsynced_changed
        + status.library.unsynced_removed;
    let admitted = status.library.total_synced_entries;
    let artwork_missing = status.library.artwork_missing;
    let rejected = status.library.failed_games + status.library.skipped_games;
    let orb_state = sync_orb_state(status, pending_changes, rejected);
    let orb_label = sync_orb_label(status, orb_state, pending_changes, rejected);
    let orb_number = sync_orb_number(status, pending_changes, rejected);
    let orb_subline = sync_orb_subline(status, pending_changes, rejected);
    let primary_label = sync_primary_action_label(status, pending_changes, rejected);
    let primary_disabled = storage_blocked || status.library.last_sync_state == "running";
    let sync_debt = if pending_changes > 0 {
        "admission"
    } else {
        "none"
    };
    let beauty_debt = if artwork_missing > 0 {
        "caveat"
    } else {
        "none"
    };
    view_shell(
        "sync",
        "",
        "",
        "",
        html! {
            section class="sync-admission-board sync-orb-board" data-sync-root="true" data-sync-orb-state=(orb_state) data-sync-debt=(sync_debt) data-beauty-debt=(beauty_debt) data-storage-health=(status.storage.health) data-storage-low=(storage_low) data-storage-blocked=(storage_blocked) data-sync-state=(status.library.last_sync_state) aria-label="Sync orb" {
                div class="sync-orb-stage ux-sync-orb-stage" data-sync-result=(sync_result_kind(status)) {
                    div class=(format!("ux-sync-orb ux-sync-orb--{}", orb_state)) aria-hidden="true" {
                        span class="ux-sync-orb-ring" {}
                        span class="ux-sync-orb-core" { (orb_number) }
                        span class="ux-sync-orb-glint" {}
                    }
                    div class="sync-orb-copy" {
                        strong class="sync-orb-verdict" { (orb_label) }
                        span class="sync-orb-subline" { (orb_subline) }
                        div class="sync-orb-lanes" aria-label="Sync debt lanes" {
                            (sync_lane_chip("Waiting", pending_changes, if pending_changes > 0 { "warn" } else { "quiet" }))
                            (sync_lane_chip("New", status.library.unsynced_added, if status.library.unsynced_added > 0 { "warn" } else { "quiet" }))
                            (sync_lane_chip("Changed", status.library.unsynced_changed, if status.library.unsynced_changed > 0 { "warn" } else { "quiet" }))
                            (sync_lane_chip("Ejected", status.library.unsynced_removed, if status.library.unsynced_removed > 0 { "bad" } else { "quiet" }))
                            (sync_lane_chip("Admitted", admitted, "good"))
                            (sync_lane_chip("Artwork missing", artwork_missing, if artwork_missing > 0 { "caveat" } else { "good" }))
                            (sync_lane_chip("Attention", rejected, if rejected > 0 { "bad" } else { "quiet" }))
                        }
                    }
                    div class="sync-orb-actions" aria-label="Sync actions" {
                        @if primary_disabled {
                            button class="btn btn--primary" type="button" data-button="primary" data-action="sync-games" data-endpoint="/api/actions/sync-games" disabled { (if storage_blocked { "Storage Full" } else { "Syncing" }) }
                        } @else {
                            (action_button(ButtonVariant::Primary, primary_label, "sync-games", "/api/actions/sync-games"))
                        }
                        button class="btn btn--secondary" type="button" data-button="secondary" data-sync-add-games="true" { "Add games" }
                    }
                }

                @if storage_blocked {
                    div class="warning sync-storage-warning" {
                        strong { "Storage is full. Free space before admitting more games." }
                        (nav_button("Storage", "storage"))
                    }
                } @else if storage_low {
                    div class="warning sync-storage-warning" {
                        strong { "Storage is low. Large admissions may fail." }
                        (nav_button("Storage", "storage"))
                    }
                }

                section id="sync-running-panel" class="sync-running-panel sync-running-panel--admission" aria-label="Admission in progress" hidden[status.library.last_sync_state != "running"] {
                    div class="sync-scanner" aria-hidden="true" {
                        span class="sync-scanner-dot" {}
                        span class="sync-scanner-line" {}
                        span class="sync-scanner-file" {}
                    }
                    div class="sync-admission-phase" aria-hidden="true" {
                        span class="sync-phase-pill" { "classify" }
                        span class="sync-phase-pill" { "admit" }
                        span class="sync-phase-pill" { "beautify" }
                    }
                }

                (sync_system_blades(status))
                (sync_admitted_shelf(status))
                div class="sync-board-proof" aria-label="Latest sync receipt" data-last-sync=(status.library.last_sync) {}
            }
            div id="console-action-message" class="message" hidden {}
        },
    )
}

fn sync_orb_state(status: &ConsoleStatus, pending_changes: u64, rejected: u64) -> &'static str {
    if status.library.last_sync_state == "running" {
        "syncing"
    } else if pending_changes > 0 {
        "waiting"
    } else if rejected > 0 {
        "attention"
    } else if status.library.artwork_missing > 0 && status.library.total_synced_entries > 0 {
        "caveat"
    } else {
        "current"
    }
}

fn sync_orb_label(
    status: &ConsoleStatus,
    orb_state: &str,
    pending_changes: u64,
    rejected: u64,
) -> String {
    match orb_state {
        "syncing" => format!(
            "Syncing {} games",
            pending_changes
                .max(status.library.total_detected_games)
                .max(1)
        ),
        "waiting" => format!("{} games waiting", pending_changes),
        "attention" => format!("{} need attention", rejected),
        "caveat" | "current" if status.library.total_synced_entries == 0 => "Add games".to_string(),
        "caveat" | "current" => "Games current".to_string(),
        _ => "Games current".to_string(),
    }
}

fn sync_orb_number(status: &ConsoleStatus, pending_changes: u64, rejected: u64) -> String {
    if status.library.last_sync_state == "running" {
        pending_changes
            .max(status.library.total_detected_games)
            .max(1)
            .to_string()
    } else if pending_changes > 0 {
        pending_changes.to_string()
    } else if rejected > 0 {
        rejected.to_string()
    } else {
        status.library.total_synced_entries.to_string()
    }
}

fn sync_orb_subline(status: &ConsoleStatus, pending_changes: u64, rejected: u64) -> String {
    if status.library.last_sync_state == "running" {
        "Admitting games into the console library".to_string()
    } else if pending_changes > 0 {
        format!(
            "{} new · {} changed · {} ejected",
            status.library.unsynced_added,
            status.library.unsynced_changed,
            status.library.unsynced_removed
        )
    } else if rejected > 0 {
        format!("{} rejected candidates need a fix", rejected)
    } else if status.library.artwork_missing > 0 && status.library.total_synced_entries > 0 {
        format!(
            "{} admitted · {} missing artwork",
            status.library.total_synced_entries, status.library.artwork_missing
        )
    } else if status.library.total_synced_entries > 0 {
        format!(
            "{} admitted · artwork complete",
            status.library.total_synced_entries
        )
    } else {
        "No games admitted yet".to_string()
    }
}

fn sync_primary_action_label(
    status: &ConsoleStatus,
    pending_changes: u64,
    rejected: u64,
) -> &'static str {
    if pending_changes > 0 || !sync_has_history(status) {
        "Sync games"
    } else if rejected > 0 {
        "Review"
    } else {
        "Check again"
    }
}

fn sync_lane_chip(label: &str, value: u64, tone: &str) -> Markup {
    html! { span class=(format!("sync-lane-chip sync-lane-chip--{}", tone)) { em { (label) } strong { (value) } } }
}

fn sync_system_blades(status: &ConsoleStatus) -> Markup {
    html! {
        div class="sync-system-blades" aria-label="Game systems admitted" {
            @if status.library.game_system_tally.is_empty() {
                article class="sync-system-blade sync-system-blade--empty" {
                    strong { "No systems yet" }
                    span { "0 admitted" }
                    div class="sync-blade-meter" style="--sync-meter:0%" {}
                }
            } @else {
                @for row in status.library.game_system_tally.iter() {
                    @let paired_percent = if row.admitted == 0 { 0 } else { ((row.artwork_paired * 100) / row.admitted).min(100) };
                    article class=(sync_system_blade_class(row)) data-system=(&row.system) {
                        div class="sync-blade-icon" aria-hidden="true" { (system_monogram(&row.system)) }
                        div class="sync-blade-copy" {
                            strong { (&row.system) }
                            span { (row.admitted) " admitted" }
                        }
                        div class="sync-blade-art" {
                            span { (row.artwork_paired) " art" }
                            @if row.artwork_missing > 0 { em { (row.artwork_missing) " missing" } }
                        }
                        div class="sync-blade-meter" style=(format!("--sync-meter:{}%", paired_percent)) {}
                    }
                }
            }
        }
    }
}

fn sync_system_blade_class(row: &GameSystemTally) -> &'static str {
    if row.artwork_missing > 0 {
        "sync-system-blade sync-system-blade--warn"
    } else {
        "sync-system-blade sync-system-blade--complete"
    }
}

fn system_monogram(system: &str) -> String {
    system
        .split_whitespace()
        .filter_map(|part| part.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
}

fn sync_admitted_shelf(status: &ConsoleStatus) -> Markup {
    html! {
        div class="sync-admitted-shelf" aria-label="Admitted game shelf" {
            @if status.library.admitted_games.is_empty() {
                div class="sync-shelf-empty" {
                    div class="sync-cover-frame sync-cover-frame--empty" aria-hidden="true" { span { "＋" } }
                    strong { "No games admitted yet" }
                    span { "Add games, then admit them into the console library." }
                }
            } @else {
                @for game in status.library.admitted_games.iter().take(10) {
                    article class="sync-game-card" data-artwork-paired=(game.artwork_paired) {
                        div class=(if game.artwork_paired { "sync-cover-frame sync-cover-frame--paired" } else { "sync-cover-frame sync-cover-frame--missing" }) aria-hidden="true" {
                            span { (system_monogram(&game.system)) }
                        }
                        div class="sync-game-card-main" {
                            strong { (&game.title) }
                            div class="sync-game-badges" {
                                span class="sync-game-badge sync-game-badge--system" { (&game.system) }
                                span class="sync-game-badge" { (&game.runner) }
                            }
                        }
                        div class="sync-game-state" {
                            span class="sync-game-admitted" { "✓ admitted" }
                            span class=(if game.artwork_paired { "sync-art sync-art--paired" } else { "sync-art sync-art--missing" }) {
                                @if game.artwork_paired { "art paired" } @else { "needs cover" }
                            }
                        }
                    }
                }
                @if status.library.admitted_games.len() > 10 {
                    div class="sync-game-more" { "+" (status.library.admitted_games.len() - 10) " more admitted games" }
                }
            }
        }
    }
}

fn sync_has_history(status: &ConsoleStatus) -> bool {
    matches!(
        status.library.last_sync_state.as_str(),
        "success" | "error" | "running"
    ) || status.library.first_sync_completed
}

fn sync_result_kind(status: &ConsoleStatus) -> &'static str {
    match status.library.last_sync_state.as_str() {
        "success" => "success",
        "error" => "error",
        "running" => "running",
        _ => "idle",
    }
}

