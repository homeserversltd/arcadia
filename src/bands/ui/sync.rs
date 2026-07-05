fn sync_view(status: &ConsoleStatus) -> Markup {
    let storage_blocked = status.storage.health == "Full";
    let storage_low = status.storage.percent_used >= 90;
    let pending_changes = status.library.unsynced_added
        + status.library.unsynced_changed
        + status.library.unsynced_removed;
    let native = status.library.gamescope_entries;
    let added = status.library.total_detected_games;
    let artwork_paired = status.library.artwork_paired_total;
    let artwork_missing = status.library.artwork_missing;
    let rejected = status.library.failed_games + status.library.skipped_games;
    let games_total = library_games_total(status);
    let orb_state = sync_orb_state(status);
    let orb_number = games_total.to_string();
    let art_progress = sync_art_progress_pct(status);
    let artwork_line = sync_artwork_lane_label(artwork_paired, added);
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
            section class="sync-admission-board sync-orb-board" data-sync-root="true" data-sync-orb-state=(orb_state) data-sync-debt=(sync_debt) data-beauty-debt=(beauty_debt) data-sync-games-total=(orb_number) data-sync-art-progress=(art_progress.to_string()) data-storage-health=(status.storage.health) data-storage-low=(storage_low) data-storage-blocked=(storage_blocked) data-sync-state=(status.library.last_sync_state) aria-label="Sync orb" {
                div class="sync-orb-stage ux-sync-orb-stage" data-sync-result=(sync_result_kind(status)) {
                    div class=(format!("ux-sync-orb ux-sync-orb--{}", orb_state)) style=(format!("--sync-art-pct:{};", art_progress)) aria-label=(format!("{games_total} games total ({native} GameScope + {added} ROMs), {art_progress} percent ROM artwork paired")) {
                        span class="ux-sync-orb-track" aria-hidden="true" {}
                        span class="ux-sync-orb-sweep" aria-hidden="true" {}
                        span class="ux-sync-orb-ring" aria-hidden="true" {}
                        span class="ux-sync-orb-core" data-bind="sync.total" { (orb_number) }
                    }
                    div class="sync-orb-copy" {
                        div class="sync-orb-lanes" aria-label="Game library counts" {
                            (sync_lane_chip("Native", native, "quiet"))
                            (sync_lane_chip("Added", added, if added > 0 { "good" } else { "quiet" }))
                            (sync_lane_chip_str("Artwork", &artwork_line, if artwork_missing > 0 { "caveat" } else if added > 0 { "good" } else { "quiet" }))
                            (sync_lane_chip("Artwork missing", artwork_missing, if artwork_missing > 0 { "caveat" } else { "good" }))
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

                section id="sync-running-panel" class="sync-running-panel sync-running-panel--admission" aria-label="Admission in progress" data-bind-show="sync.running" hidden[status.library.last_sync_state != "running"] {
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

fn sync_orb_state(status: &ConsoleStatus) -> &'static str {
    if status.library.last_sync_state == "running" {
        "syncing"
    } else if library_games_total(status) == 0 {
        "idle"
    } else if status.library.artwork_missing > 0 {
        "caveat"
    } else {
        "current"
    }
}

fn sync_art_progress_pct(status: &ConsoleStatus) -> u8 {
    let added = status.library.total_detected_games;
    if added == 0 {
        return 0;
    }
    ((status.library.artwork_paired_total.saturating_mul(100)) / added)
        .min(100) as u8
}

fn sync_artwork_lane_label(artwork_paired: u64, added: u64) -> String {
    if added == 0 {
        "0".to_string()
    } else {
        format!("{artwork_paired} / {added}")
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
    let bind = match label {
        "Native" => "sync.native",
        "Added" => "sync.added",
        "Artwork missing" => "sync.artworkMissing",
        _ => "sync.total",
    };
    html! { span class=(format!("sync-lane-chip sync-lane-chip--{}", tone)) { em { (label) } strong data-bind=(bind) { (value) } } }
}

fn sync_lane_chip_str(label: &str, value: &str, tone: &str) -> Markup {
    let bind = if label == "Artwork" { "sync.artwork" } else { "sync.total" };
    html! { span class=(format!("sync-lane-chip sync-lane-chip--{}", tone)) { em { (label) } strong data-bind=(bind) { (value) } } }
}

fn sync_system_blades(status: &ConsoleStatus) -> Markup {
    html! {
        div class="sync-system-blades" aria-label="Game systems admitted" data-bind-each="sync.systems" {
            template {
                article class="sync-system-blade" data-bind-class="tone" data-system="" {
                    div class="sync-blade-icon" aria-hidden="true" data-bind="monogram" {}
                    div class="sync-blade-copy" { strong data-bind="system" {} span data-bind="admitted" {} }
                    div class="sync-blade-meter" style="--sync-meter:0%" {}
                    em data-bind="artworkPaired" {}
                }
            }
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
                @for game in status.library.admitted_games.iter() {
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
            }
        }
    }
}

fn library_games_total(status: &ConsoleStatus) -> u64 {
    status.library.total_detected_games
}

fn library_games_total_tip(status: &ConsoleStatus, detail: &str) -> String {
    let total = library_games_total(status);
    let artwork = library_artwork_lane_label(status);
    format!("{total} games · {artwork} artwork · {detail}")
}

fn library_artwork_lane_label(status: &ConsoleStatus) -> String {
    sync_artwork_lane_label(
        status.library.artwork_paired_total,
        status.library.total_detected_games,
    )
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