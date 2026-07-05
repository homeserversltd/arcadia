#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiRootObject {
    pub schema: &'static str,
    pub kind: &'static str,
    pub id: &'static str,
    pub generated_at_unix: u64,
    pub product: String,
    pub canonical_url: String,
    pub children: Vec<ApiObjectNode>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiObjectNode {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub state: String,
    pub route: Option<String>,
    pub summary: serde_json::Value,
    pub metrics: Vec<ApiMetric>,
    pub data: serde_json::Value,
    pub children: Vec<ApiObjectNode>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiMetric {
    pub id: String,
    pub label: String,
    pub value: serde_json::Value,
    pub unit: Option<String>,
    pub state: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiLivingStateDocument {
    pub schema: &'static str,
    pub kind: &'static str,
    pub id: &'static str,
    pub generated_at_unix: u64,
    pub status: ConsoleStatus,
    pub home: ApiHomeState,
    pub sync: ApiSyncState,
    pub storage_pane: ApiStoragePaneState,
    pub storage: StorageStatus,
    pub storage_summary: StorageStatus,
    pub network: NetworkState,
    pub ai: LocalAIState,
    pub controllers: ControllerStatus,
    pub system: SystemAdminStatus,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiSyncState {
    pub state: String,
    pub state_line: String,
    pub phase_line: &'static str,
    pub running: bool,
    pub result: &'static str,
    pub orb: &'static str,
    pub debt: &'static str,
    pub beauty: &'static str,
    pub storage_health: String,
    pub storage_low: bool,
    pub storage_blocked: bool,
    pub total: String,
    pub native: String,
    pub added: String,
    pub artwork: String,
    pub artwork_missing: String,
    pub artwork_progress: String,
    pub admitted: String,
    pub primary_label: &'static str,
    pub disabled_label: &'static str,
    pub systems: Vec<ApiSyncSystemState>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiSyncSystemState {
    pub system: String,
    pub admitted: String,
    pub artwork_paired: String,
    pub artwork_missing: String,
    pub meter: String,
    pub tone: &'static str,
    pub monogram: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiStoragePaneState {
    pub hero: ApiStoragePaneHeroState,
    pub capacity: ApiStoragePaneCapacityState,
    pub mismatch: ApiStoragePaneMismatchState,
    pub categories: ApiStoragePaneCategoriesState,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiStoragePaneHeroState {
    pub total: String,
    pub used: String,
    pub free: String,
    pub scan: String,
    pub percent: String,
    pub health: &'static str,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiStoragePaneCapacityState {
    pub used_line: String,
    pub segments: ApiStoragePaneSegmentsState,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiStoragePaneSegmentsState {
    pub games: ApiStoragePaneStyleState,
    pub artwork: ApiStoragePaneStyleState,
    pub ai: ApiStoragePaneStyleState,
    pub updates: ApiStoragePaneStyleState,
    pub logs: ApiStoragePaneStyleState,
    pub temporary: ApiStoragePaneStyleState,
    pub system: ApiStoragePaneStyleState,
    pub other: ApiStoragePaneStyleState,
    pub free: ApiStoragePaneStyleState,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiStoragePaneStyleState {
    pub width: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiStoragePaneMismatchState {
    pub visible: bool,
    pub state: &'static str,
    pub title: String,
    pub copy: String,
    pub filesystem_used: String,
    pub category_scan: String,
    pub other: String,
    pub scan_time: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiStoragePaneCategoriesState {
    pub games: ApiStoragePaneCategoryState,
    pub artwork: ApiStoragePaneCategoryState,
    pub ai: ApiStoragePaneCategoryState,
    pub updates: ApiStoragePaneCategoryState,
    pub logs: ApiStoragePaneCategoryState,
    pub temporary: ApiStoragePaneCategoryState,
    pub system: ApiStoragePaneCategoryState,
    pub other: ApiStoragePaneCategoryState,
    pub free: ApiStoragePaneCategoryState,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiStoragePaneCategoryState {
    pub size: String,
    pub percent: String,
    pub badge: String,
    pub state: &'static str,
    pub bar: ApiStoragePaneStyleState,
}


#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiHomeState {
    pub priority: ApiHomePriorityState,
    pub storage: ApiHomeStorageState,
    pub games: ApiHomeGamesState,
    pub network: ApiHomeNetworkState,
    pub updates: ApiHomeUpdatesState,
    pub ai: ApiHomeAiState,
    pub warning: ApiHomeWarningState,
    pub session: ApiHomeSessionState,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiHomePriorityState {
    pub visible: bool,
    pub state: String,
    pub badge: &'static str,
    pub tone: &'static str,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiHomeStorageState {
    pub state: &'static str,
    pub attention: bool,
    pub games_size: String,
    pub ai_size: String,
    pub everything_else_size: String,
    pub free_size: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiHomeGamesState {
    pub state: String,
    pub attention: bool,
    pub total: String,
    pub artwork: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiHomeNetworkState {
    pub state: &'static str,
    pub attention: bool,
    pub headline: &'static str,
    pub console_reachability: &'static str,
    pub ai_reachability: &'static str,
    pub internet_reachability: &'static str,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiHomeUpdatesState {
    pub state: String,
    pub attention: bool,
    pub readiness_ratio: String,
    pub pending_updates: String,
    pub last_ran: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiHomeAiState {
    pub state: String,
    pub attention: bool,
    pub model: String,
    pub load: &'static str,
    pub activity: &'static str,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiHomeWarningState {
    pub visible: bool,
    pub title: &'static str,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiHomeSessionState {
    pub visible: bool,
    pub state: &'static str,
}

const HOME_TELEMETRY_TOPIC: &str = "home.load";
const HOME_TELEMETRY_CADENCE_SECONDS: u64 = 1;
const HOME_TELEMETRY_RENEW_SECONDS: u64 = 10;
const HOME_TELEMETRY_IDLE_TIMEOUT_SECONDS: u64 = 30;

static HOME_TELEMETRY_LEASE_COUNTER: AtomicU64 = AtomicU64::new(1);
static HOME_TELEMETRY_LEASES: OnceLock<Mutex<HashMap<String, HomeTelemetryLease>>> = OnceLock::new();
static LAST_DISK_IO_SAMPLE: OnceLock<Mutex<Option<DiskIoSample>>> = OnceLock::new();
static LAST_CPU_USAGE_SAMPLE: OnceLock<Mutex<Option<CpuUsageSample>>> = OnceLock::new();

#[derive(Clone, Copy)]
struct DiskIoSample {
    sectors_read: u64,
    sectors_written: u64,
    sampled_at_millis: u128,
}

#[derive(Clone, Copy)]
struct CpuUsageSample {
    busy_jiffies: u64,
    total_jiffies: u64,
}

#[derive(Clone)]
struct HomeTelemetryLease {
    lease_id: String,
    topic: String,
    joined_at_unix: u64,
    last_contact_unix: u64,
    expires_at_unix: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HomeTelemetryRenewRequest {
    lease_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HomeTelemetryLeaseResponse {
    schema: &'static str,
    kind: &'static str,
    lease_id: String,
    topic: String,
    joined_at_unix: u64,
    last_contact_unix: u64,
    renew_after_seconds: u64,
    expires_at_unix: u64,
    active: bool,
}

fn home_telemetry_leases() -> &'static Mutex<HashMap<String, HomeTelemetryLease>> {
    HOME_TELEMETRY_LEASES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn home_telemetry_response(lease: &HomeTelemetryLease, active: bool) -> HomeTelemetryLeaseResponse {
    HomeTelemetryLeaseResponse {
        schema: "arcadia.api.root.lease.v1",
        kind: "homeTelemetryLease",
        lease_id: lease.lease_id.clone(),
        topic: lease.topic.clone(),
        joined_at_unix: lease.joined_at_unix,
        last_contact_unix: lease.last_contact_unix,
        renew_after_seconds: HOME_TELEMETRY_RENEW_SECONDS,
        expires_at_unix: lease.expires_at_unix,
        active,
    }
}

fn home_telemetry_create_lease() -> HomeTelemetryLeaseResponse {
    let now = now_unix_seconds();
    let lease_seq = HOME_TELEMETRY_LEASE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let lease = HomeTelemetryLease {
        lease_id: format!("home-load-{lease_seq}"),
        topic: HOME_TELEMETRY_TOPIC.to_string(),
        joined_at_unix: now,
        last_contact_unix: now,
        expires_at_unix: now + HOME_TELEMETRY_IDLE_TIMEOUT_SECONDS,
    };
    let response = home_telemetry_response(&lease, true);
    if let Ok(mut leases) = home_telemetry_leases().lock() {
        leases.retain(|_, lease| lease.expires_at_unix >= now);
        leases.insert(lease.lease_id.clone(), lease);
    }
    response
}

fn home_telemetry_renew_lease(lease_id: &str) -> Option<HomeTelemetryLeaseResponse> {
    let now = now_unix_seconds();
    let mut leases = home_telemetry_leases().lock().ok()?;
    leases.retain(|_, lease| lease.expires_at_unix >= now);
    let lease = leases.get_mut(lease_id)?;
    lease.last_contact_unix = now;
    lease.expires_at_unix = now + HOME_TELEMETRY_IDLE_TIMEOUT_SECONDS;
    Some(home_telemetry_response(lease, true))
}

fn home_telemetry_lease_status(lease_id: &str) -> Option<HomeTelemetryLeaseResponse> {
    let now = now_unix_seconds();
    let mut leases = home_telemetry_leases().lock().ok()?;
    leases.retain(|_, lease| lease.expires_at_unix >= now);
    let lease = leases.get(lease_id)?;
    Some(home_telemetry_response(lease, lease.expires_at_unix >= now))
}

fn home_telemetry_drop_lease(lease_id: &str) {
    if let Ok(mut leases) = home_telemetry_leases().lock() {
        leases.remove(lease_id);
    }
}

async fn api_root_route(State(state): State<Arc<AppState>>) -> Json<ApiRootObject> {
    Json(api_root_object(&state))
}

async fn api_root_events_renew_route(
    Json(request): Json<HomeTelemetryRenewRequest>,
) -> impl IntoResponse {
    match home_telemetry_renew_lease(&request.lease_id) {
        Some(lease) => (StatusCode::OK, Json(lease)).into_response(),
        None => (
            StatusCode::GONE,
            Json(serde_json::json!({
                "schema": "arcadia.api.root.lease.v1",
                "kind": "homeTelemetryLeaseExpired",
                "leaseId": request.lease_id,
                "active": false,
            })),
        )
            .into_response(),
    }
}

async fn api_root_events_route(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let lease = home_telemetry_create_lease();
    let lease_id = lease.lease_id.clone();
    let stream = async_stream::stream! {
        let snapshot = api_root_object(&state);
        let snapshot_json = serde_json::to_string(&snapshot)
            .unwrap_or_else(|_| "{}".to_string());
        yield Ok(Event::default()
            .event("snapshot")
            .id(snapshot.generated_at_unix.to_string())
            .data(snapshot_json));

        let lease_json = serde_json::to_string(&lease)
            .unwrap_or_else(|_| "{}".to_string());
        yield Ok(Event::default().event("lease").data(lease_json));

        let mut tick = tokio::time::interval(Duration::from_secs(HOME_TELEMETRY_CADENCE_SECONDS));
        loop {
            tick.tick().await;
            let Some(status) = home_telemetry_lease_status(&lease_id) else {
                let expired = serde_json::json!({
                    "schema": "arcadia.api.root.lease.v1",
                    "kind": "homeTelemetryLeaseExpired",
                    "leaseId": lease_id,
                    "topic": HOME_TELEMETRY_TOPIC,
                    "generatedAtUnix": now_unix_seconds(),
                    "active": false,
                });
                yield Ok(Event::default().event("expired").data(expired.to_string()));
                break;
            };

            let root = api_root_telemetry_tick(&state);
            let payload = serde_json::to_string(&root)
                .unwrap_or_else(|_| "{}".to_string());
            yield Ok(Event::default()
                .event("root")
                .id(root.generated_at_unix.to_string())
                .data(payload));

            let living_state = api_living_state_document(&state);
            let living_state_json = serde_json::to_string(&living_state)
                .unwrap_or_else(|_| "{}".to_string());
            yield Ok(Event::default()
                .event("state")
                .id(living_state.generated_at_unix.to_string())
                .data(living_state_json));

            let heartbeat = serde_json::to_string(&status)
                .unwrap_or_else(|_| "{}".to_string());
            yield Ok(Event::default().event("heartbeat").data(heartbeat));
        }
        home_telemetry_drop_lease(&lease_id);
    };

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("arcadia-home-telemetry"),
    )
}

fn api_root_object(state: &AppState) -> ApiRootObject {
    let status = console_status(state);
    api_root_object_from_status(state, &status)
}

fn api_root_object_from_status(_state: &AppState, status: &ConsoleStatus) -> ApiRootObject {
    ApiRootObject {
        schema: "arcadia.api.root.v1",
        kind: "arcadia-root",
        id: "arcadia",
        generated_at_unix: now_unix_seconds(),
        product: status.product.clone(),
        canonical_url: status.canonical_url.clone(),
        children: vec![
            api_appliance_node(status),
            api_storage_node(status),
            api_telemetry_node(),
            api_routes_node(),
        ],
    }
}

fn api_root_telemetry_tick(state: &AppState) -> ApiRootObject {
    ApiRootObject {
        schema: "arcadia.api.root.v1",
        kind: "arcadia-root",
        id: "arcadia",
        generated_at_unix: now_unix_seconds(),
        product: state.product.clone(),
        canonical_url: state.canonical_url.clone(),
        children: vec![api_telemetry_node()],
    }
}

fn api_living_state_document(state: &AppState) -> ApiLivingStateDocument {
    let status = console_status(state);
    let storage = storage_status();
    ApiLivingStateDocument {
        schema: "arcadia.api.state.v1",
        kind: "arcadiaLivingState",
        id: "arcadia-state",
        generated_at_unix: now_unix_seconds(),
        home: api_home_state(&status),
        sync: api_sync_state(&status),
        storage_pane: api_storage_pane_state(&status),
        status,
        storage: storage.clone(),
        storage_summary: storage,
        network: network_state(state),
        ai: local_ai_state(state),
        controllers: controller_status_api(),
        system: system_admin_status(&network_status(), &hostname()),
    }
}


fn api_storage_pane_state(status: &ConsoleStatus) -> ApiStoragePaneState {
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
    let diagnostics_count = status.storage.diagnostics.missing_dirs.len()
        + status.storage.diagnostics.permission_errors.len()
        + status.storage.diagnostics.warnings.len()
        + status.storage.diagnostics.overlap_warnings.len()
        + status.storage.diagnostics.category_scan_errors.len();
    let mismatch_visible = diagnostics_count > 0;
    let mismatch_copy = if mismatch_visible {
        format!(
            "Managed storage categories overlap or could not be fully scanned. Filesystem used {}. Managed categories classify {}; accounting total is {}.",
            status.storage.used,
            human_size_or_zero(classified_bytes),
            human_size_or_zero(accounted_bytes)
        )
    } else {
        String::new()
    };
    ApiStoragePaneState {
        hero: ApiStoragePaneHeroState {
            total: status.storage.total.clone(),
            used: status.storage.used.clone(),
            free: status.storage.free.clone(),
            scan: api_storage_scan_label(&status.storage.scanned_at),
            percent: status.storage.percent.clone(),
            health: status.storage.health,
        },
        capacity: ApiStoragePaneCapacityState {
            used_line: format!("{} used of {}", status.storage.used, status.storage.total),
            segments: ApiStoragePaneSegmentsState {
                games: api_storage_style_width(status.storage.games.percent_of_total, status.storage.games.bytes),
                artwork: api_storage_style_width(status.storage.artwork.percent_of_total, status.storage.artwork.bytes),
                ai: api_storage_style_width(status.storage.ai_models.percent_of_total, status.storage.ai_models.bytes),
                updates: api_storage_style_width(status.storage.categories.updates.percent_of_total, status.storage.categories.updates.bytes),
                logs: api_storage_style_width(status.storage.categories.logs.percent_of_total, status.storage.categories.logs.bytes),
                temporary: api_storage_style_width(status.storage.categories.temporary.percent_of_total, status.storage.categories.temporary.bytes),
                system: api_storage_style_width(status.storage.categories.system.percent_of_total, status.storage.categories.system.bytes),
                other: api_storage_style_width(status.storage.other.percent_of_total, status.storage.other.bytes),
                free: ApiStoragePaneStyleState { width: format!("{}%", 100u8.saturating_sub(status.storage.percent_used)) },
            },
        },
        mismatch: ApiStoragePaneMismatchState {
            visible: mismatch_visible,
            state: if mismatch_visible { "warning" } else { "ok" },
            title: if mismatch_visible { "Storage mismatch detected".to_string() } else { String::new() },
            copy: mismatch_copy,
            filesystem_used: status.storage.used.clone(),
            category_scan: human_size_or_zero(classified_bytes),
            other: status.storage.other.size.clone(),
            scan_time: format!("{} ms", status.storage.diagnostics.scan_duration_ms),
        },
        categories: ApiStoragePaneCategoriesState {
            games: api_storage_pane_category(&status.storage.games),
            artwork: api_storage_pane_category(&status.storage.artwork),
            ai: api_ai_storage_pane_category(&status.storage.ai_models),
            updates: api_storage_pane_category(&status.storage.categories.updates),
            logs: api_storage_pane_category(&status.storage.categories.logs),
            temporary: api_storage_pane_category(&status.storage.categories.temporary),
            system: api_storage_pane_category(&status.storage.categories.system),
            other: api_storage_pane_category(&status.storage.other),
            free: ApiStoragePaneCategoryState {
                size: status.storage.free.clone(),
                percent: api_percent_label(100u8.saturating_sub(status.storage.percent_used), status.storage.free_bytes),
                badge: "Available".to_string(),
                state: "available",
                bar: ApiStoragePaneStyleState { width: format!("{}%", 100u8.saturating_sub(status.storage.percent_used)) },
            },
        },
    }
}

fn api_storage_pane_category(category: &StorageCategoryStatus) -> ApiStoragePaneCategoryState {
    ApiStoragePaneCategoryState {
        size: category.size.clone(),
        percent: api_percent_label(category.percent_of_total, category.bytes),
        badge: api_title_case_state_like(&category.state).to_string(),
        state: api_storage_state_class(&category.state),
        bar: api_storage_style_width(category.percent_of_total, category.bytes),
    }
}

fn api_ai_storage_pane_category(category: &AiModelStorageStatus) -> ApiStoragePaneCategoryState {
    ApiStoragePaneCategoryState {
        size: category.size.clone(),
        percent: api_percent_label(category.percent_of_total, category.bytes),
        badge: category.meta.clone(),
        state: "available",
        bar: api_storage_style_width(category.percent_of_total, category.bytes),
    }
}

fn api_storage_style_width(percent: u8, bytes: u64) -> ApiStoragePaneStyleState {
    ApiStoragePaneStyleState { width: format!("{}%", percent.max(if bytes == 0 { 0 } else { 1 })) }
}

fn api_storage_scan_label(scanned_at: &str) -> String {
    if scanned_at.contains('T') { "just now".to_string() } else { scanned_at.to_string() }
}

fn api_storage_state_class(state: &str) -> &'static str {
    match state {
        "ok" => "available",
        "warning" => "starting",
        "unknown" => "unknown",
        _ => "unknown",
    }
}

fn api_title_case_state_like(state: &str) -> &'static str {
    match state {
        "ok" => "Available",
        "warning" => "Review",
        "unknown" => "Unknown",
        "available" => "Available",
        _ => "Unknown",
    }
}

fn api_percent_label(percent: u8, bytes: u64) -> String {
    if bytes > 0 && percent == 0 { "<1%".to_string() } else { format!("{}%", percent) }
}

fn human_size_or_zero(bytes: u64) -> String {
    if bytes == 0 { "0 B".to_string() } else { human_size(bytes) }
}

fn api_sync_state(status: &ConsoleStatus) -> ApiSyncState {
    let pending_changes = status.library.unsynced_added
        + status.library.unsynced_changed
        + status.library.unsynced_removed;
    let rejected = status.library.failed_games + status.library.skipped_games;
    let added = status.library.total_detected_games;
    let artwork_progress = api_sync_art_progress_pct(status);
    let storage_blocked = status.storage.health == "Full";
    let storage_low = status.storage.percent_used >= 90;
    let running = status.library.last_sync_state == "running";
    ApiSyncState {
        state: status.library.last_sync_state.clone(),
        state_line: api_sync_state_line(status, pending_changes, rejected),
        phase_line: if running { "classify admit beautify" } else { "ready" },
        running,
        result: api_sync_result_kind(status),
        orb: api_sync_orb_state(status),
        debt: if pending_changes > 0 { "admission" } else { "none" },
        beauty: if status.library.artwork_missing > 0 { "caveat" } else { "none" },
        storage_health: status.storage.health.to_string(),
        storage_low,
        storage_blocked,
        total: api_sync_library_games_total(status).to_string(),
        native: status.library.gamescope_entries.to_string(),
        added: added.to_string(),
        artwork: api_sync_artwork_lane_label(status.library.artwork_paired_total, added),
        artwork_missing: status.library.artwork_missing.to_string(),
        artwork_progress: artwork_progress.to_string(),
        admitted: status.library.total_synced_entries.to_string(),
        primary_label: api_sync_primary_action_label(status, pending_changes, rejected),
        disabled_label: if storage_blocked { "Storage Full" } else { "Syncing" },
        systems: status.library.game_system_tally.iter().map(api_sync_system_state).collect(),
    }
}

fn api_sync_system_state(row: &GameSystemTally) -> ApiSyncSystemState {
    let meter = if row.admitted == 0 { 0 } else { ((row.artwork_paired * 100) / row.admitted).min(100) };
    ApiSyncSystemState {
        system: row.system.clone(),
        admitted: format!("{} admitted", row.admitted),
        artwork_paired: format!("{} art", row.artwork_paired),
        artwork_missing: if row.artwork_missing > 0 { format!("{} missing", row.artwork_missing) } else { String::new() },
        meter: format!("{meter}%"),
        tone: if row.artwork_missing > 0 { "warn" } else { "complete" },
        monogram: api_sync_system_monogram(&row.system),
    }
}

fn api_sync_state_line(status: &ConsoleStatus, pending_changes: u64, rejected: u64) -> String {
    match status.library.last_sync_state.as_str() {
        "running" => "Syncing".to_string(),
        "error" => "Review".to_string(),
        _ if pending_changes > 0 => format!("{pending_changes} waiting"),
        _ if rejected > 0 => "Review".to_string(),
        _ if !api_sync_has_history(status) => "Not scanned yet".to_string(),
        _ => "Ready".to_string(),
    }
}

fn api_sync_orb_state(status: &ConsoleStatus) -> &'static str {
    if status.library.last_sync_state == "running" {
        "syncing"
    } else if api_sync_library_games_total(status) == 0 {
        "idle"
    } else if status.library.artwork_missing > 0 {
        "caveat"
    } else {
        "current"
    }
}

fn api_sync_art_progress_pct(status: &ConsoleStatus) -> u8 {
    let added = status.library.total_detected_games;
    if added == 0 { return 0; }
    ((status.library.artwork_paired_total.saturating_mul(100)) / added).min(100) as u8
}

fn api_sync_artwork_lane_label(artwork_paired: u64, added: u64) -> String {
    if added == 0 { "0".to_string() } else { format!("{artwork_paired} / {added}") }
}

fn api_sync_primary_action_label(status: &ConsoleStatus, pending_changes: u64, rejected: u64) -> &'static str {
    if pending_changes > 0 || !api_sync_has_history(status) {
        "Sync games"
    } else if rejected > 0 {
        "Review"
    } else {
        "Check again"
    }
}

fn api_sync_has_history(status: &ConsoleStatus) -> bool {
    matches!(status.library.last_sync_state.as_str(), "success" | "error" | "running")
        || status.library.first_sync_completed
}

fn api_sync_result_kind(status: &ConsoleStatus) -> &'static str {
    match status.library.last_sync_state.as_str() {
        "success" => "success",
        "error" => "error",
        "running" => "running",
        _ => "idle",
    }
}

fn api_sync_library_games_total(status: &ConsoleStatus) -> u64 {
    status.library.total_detected_games
}

fn api_sync_system_monogram(system: &str) -> String {
    system
        .split_whitespace()
        .filter_map(|part| part.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
}

fn api_home_state(status: &ConsoleStatus) -> ApiHomeState {
    let priority = api_home_priority_state(status);
    let storage_attention = status.storage.percent_used >= 90;
    let pending_changes = status.library.unsynced_added
        + status.library.unsynced_changed
        + status.library.unsynced_removed;
    let games_attention = status.library.last_sync_state == "error"
        || pending_changes > 0
        || status.library.sync_needed;
    let updates_ready = status.updates.modules.iter().filter(|module| module.enabled && module.present).count();
    let updates_enabled = status.updates.modules.iter().filter(|module| module.enabled).count();
    let updates_attention = status.updates.pending_updates > 0
        || updates_ready < updates_enabled
        || !status.updates.check_ok && status.updates.check_missing_signal != "not-checked"
        || !status.updates.suite_ok;
    let ai_attention = status.local_ai.load_state == "error";
    ApiHomeState {
        priority,
        storage: ApiHomeStorageState {
            state: if storage_attention { "attention" } else { status.storage.health },
            attention: storage_attention,
            games_size: status.storage.games.size.clone(),
            ai_size: status.storage.ai_models.size.clone(),
            everything_else_size: human_size(status.storage.artwork.bytes.saturating_add(status.storage.other.bytes)),
            free_size: status.storage.free.clone(),
        },
        games: ApiHomeGamesState {
            state: if games_attention { "attention".to_string() } else { status.library.last_sync_state.clone() },
            attention: games_attention,
            total: status.library.total_detected_games.to_string(),
            artwork: format!("{} / {}", status.library.artwork_paired_total, status.library.total_detected_games),
        },
        network: ApiHomeNetworkState {
            state: if status.network.online { "online" } else { "offline" },
            attention: !status.network.online,
            headline: if status.network.online { "Online" } else { "Offline" },
            console_reachability: if status.network.online { "Online" } else { "Offline" },
            ai_reachability: if status.network.lan_ai_reachable { "Online" } else { "Offline" },
            internet_reachability: if matches!(status.network.internet_reachable, Some(true)) { "Online" } else { "Offline" },
        },
        updates: ApiHomeUpdatesState {
            state: status.updates.state.clone(),
            attention: updates_attention,
            readiness_ratio: format!("{updates_ready}/{updates_enabled}"),
            pending_updates: status.updates.pending_updates.to_string(),
            last_ran: status.updates.last_update_run.clone(),
        },
        ai: ApiHomeAiState {
            state: status.local_ai.load_state.clone(),
            attention: ai_attention,
            model: api_home_local_ai_model_name(status),
            load: api_home_local_ai_load_label(&status.local_ai.load_state),
            activity: api_home_local_ai_activity_label(status),
        },
        warning: ApiHomeWarningState {
            visible: status.library.last_sync_state == "error" || status.local_ai.load_state == "error",
            title: if status.library.last_sync_state == "error" { "Sync failed" } else { "Local AI error" },
        },
        session: ApiHomeSessionState {
            visible: status.arcadia.service != "running",
            state: status.arcadia.service,
        },
    }
}

fn api_home_priority_state(status: &ConsoleStatus) -> ApiHomePriorityState {
    let priority: Option<(String, &'static str)> = if !status.network.online {
        Some(("Network offline".to_string(), "bad"))
    } else if status.storage.percent_used >= 90 {
        Some((format!("Storage low: {} free", status.storage.free), "warn"))
    } else if status.library.last_sync_state == "error" {
        Some(("Sync failed".to_string(), "bad"))
    } else if !status.library.first_sync_completed && status.library.last_sync_state != "success" {
        Some(("First sync waiting".to_string(), "idle"))
    } else if status.library.sync_needed {
        let changes = status.library.unsynced_added
            + status.library.unsynced_changed
            + status.library.unsynced_removed;
        Some((format!("{} changes waiting for sync", changes), "warn"))
    } else if status.updates.state == "available" {
        Some(("Update available".to_string(), "warn"))
    } else if status.local_ai.load_state == "error" {
        Some(("Local AI error".to_string(), "bad"))
    } else {
        None
    };
    if let Some((state, tone)) = priority {
        ApiHomePriorityState {
            visible: true,
            state,
            badge: if tone == "idle" { "Waiting" } else { "Attention" },
            tone,
        }
    } else {
        ApiHomePriorityState { visible: false, state: String::new(), badge: "", tone: "idle" }
    }
}

fn api_home_local_ai_model_name(status: &ConsoleStatus) -> String {
    status
        .local_ai
        .loaded_model_name
        .clone()
        .or_else(|| status.local_ai.selected_model_name.clone())
        .unwrap_or_else(|| {
            if status.local_ai.available_models.is_empty() {
                "No models installed".to_string()
            } else {
                "No model selected".to_string()
            }
        })
}

fn api_home_local_ai_load_label(load_state: &str) -> &'static str {
    match load_state {
        "hot" => "Hot",
        "cold" => "Cold",
        "unloaded" => "Unloaded",
        "error" => "Error",
        _ => "Unknown",
    }
}

fn api_home_local_ai_activity_label(status: &ConsoleStatus) -> &'static str {
    if status.local_ai.load_state == "error" {
        return "Error";
    }
    if status.local_ai.load_state == "hot" && status.local_ai.lan_inference_enabled {
        return "Actively working";
    }
    match status.local_ai.load_state.as_str() {
        "hot" => "Idle",
        "cold" => "Idle",
        "unloaded" if status.local_ai.available_models.is_empty() => "Idle",
        "unloaded" => "Not loaded",
        _ => "Idle",
    }
}

fn api_appliance_node(status: &ConsoleStatus) -> ApiObjectNode {
    ApiObjectNode {
        id: "appliance".to_string(),
        kind: "object".to_string(),
        title: "Appliance".to_string(),
        state: status.arcadia.service.to_string(),
        route: Some("/api/status".to_string()),
        summary: serde_json::json!({
            "product": status.product,
            "hostname": status.identity.hostname,
            "version": status.identity.version,
            "network": status.network.connection_type,
            "arcadia": status.arcadia.service,
        }),
        metrics: vec![
            api_metric("uptime", "Machine uptime", status.runtime.machine_uptime_seconds, Some("seconds"), None),
            api_metric("arcadiaUptime", "Arcadia uptime", status.runtime.arcadia_uptime_seconds, Some("seconds"), None),
        ],
        data: serde_json::json!({
            "identity": status.identity,
            "runtime": status.runtime,
            "network": status.network,
            "system": status.system,
        }),
        children: vec![
            api_leaf("network", "Network", "object", &status.network.connection_type, "/api/network/state", serde_json::json!(status.network)),
            api_leaf("system", "System", "object", "available", "/api/system/status", serde_json::json!(status.system)),
            api_leaf("localAi", "Local AI", "object", &status.local_ai.load_state, "/api/ai/state", serde_json::json!(status.local_ai)),
            api_leaf("controllers", "Controllers", "object", &status.controllers.state, "/api/controllers/state", serde_json::json!(status.controllers)),
        ],
    }
}

fn api_storage_node(status: &ConsoleStatus) -> ApiObjectNode {
    ApiObjectNode {
        id: "storage".to_string(),
        kind: "object".to_string(),
        title: "Storage".to_string(),
        state: status.storage.health.to_string(),
        route: Some("/api/storage/state".to_string()),
        summary: serde_json::json!({
            "health": status.storage.health,
            "used": status.storage.used,
            "free": status.storage.free,
            "percentUsed": status.storage.percent_used,
            "detectedGames": status.library.total_detected_games,
            "gamescopeEntries": status.library.gamescope_entries,
            "artworkComplete": status.library.artwork_complete,
            "artworkMissing": status.library.artwork_missing,
        }),
        metrics: vec![
            api_metric("totalBytes", "Total", status.storage.total_bytes, Some("bytes"), None),
            api_metric("usedBytes", "Used", status.storage.used_bytes, Some("bytes"), None),
            api_metric("freeBytes", "Free", status.storage.free_bytes, Some("bytes"), None),
            api_metric("percentUsed", "Used", status.storage.percent_used, Some("percent"), Some(status.storage.health)),
        ],
        data: serde_json::json!({
            "storage": status.storage,
            "library": status.library,
        }),
        children: vec![
            api_games_node(status),
            api_artwork_node(status),
            api_leaf("aiModels", "AI Models", "storage-category", &status.storage.categories.ai_models.state, "/api/storage/ai-models", serde_json::json!({"summary": status.storage.categories.ai_models, "models": status.storage.ai_model_files})),
            api_leaf("cleanup", "Cleanup", "storage-control", "available", "/api/storage/cleanup", serde_json::json!(status.storage.cleanup)),
            api_leaf("locations", "Locations", "storage-registry", "available", "/api/storage/locations", serde_json::json!(status.storage.registry)),
            api_leaf("diagnostics", "Diagnostics", "storage-diagnostics", "available", "/api/storage/diagnostics", serde_json::json!(status.storage.diagnostics)),
        ],
    }
}

fn api_games_node(status: &ConsoleStatus) -> ApiObjectNode {
    ApiObjectNode {
        id: "games".to_string(),
        kind: "storage-domain".to_string(),
        title: "Games".to_string(),
        state: status.storage.categories.games.state.clone(),
        route: Some("/api/storage/games".to_string()),
        summary: serde_json::json!({
            "roms": status.library.total_detected_games,
            "files": status.library.detected_files,
            "gamescopeEntries": status.library.gamescope_entries,
            "profiles": status.library.gamescope_profiles.len(),
        }),
        metrics: vec![
            api_metric("roms", "Playable ROMs", status.library.total_detected_games, Some("count"), None),
            api_metric("gamescopeEntries", "GameScope entries", status.library.gamescope_entries, Some("count"), None),
        ],
        data: serde_json::json!({
            "summary": status.storage.categories.games,
            "folders": status.storage.game_folders,
            "gamescopeProfiles": status.library.gamescope_profiles,
            "gamescopeInstalledGames": status.library.gamescope_installed_games,
        }),
        children: status
            .storage
            .game_folders
            .iter()
            .map(|folder| {
                api_leaf(
                    &folder.platform.to_ascii_lowercase(),
                    &folder.display_name,
                    "game-platform",
                    if folder.file_count > 0 { "present" } else { "empty" },
                    &format!("/api/storage/games/{}", folder.platform.to_ascii_lowercase()),
                    serde_json::json!(folder),
                )
            })
            .collect(),
    }
}

fn api_artwork_node(status: &ConsoleStatus) -> ApiObjectNode {
    ApiObjectNode {
        id: "artwork".to_string(),
        kind: "storage-domain".to_string(),
        title: "Artwork".to_string(),
        state: status.storage.categories.artwork.state.clone(),
        route: Some("/api/storage/artwork".to_string()),
        summary: serde_json::json!({
            "status": status.library.artwork_status,
            "complete": status.library.artwork_complete,
            "missing": status.library.artwork_missing,
            "stores": status.storage.artwork_stores.len(),
        }),
        metrics: vec![
            api_metric("complete", "Complete artwork", status.library.artwork_complete, Some("count"), None),
            api_metric("missing", "Missing artwork", status.library.artwork_missing, Some("count"), None),
        ],
        data: serde_json::json!({
            "summary": status.storage.categories.artwork,
            "stores": status.storage.artwork_stores,
        }),
        children: status
            .storage
            .artwork_stores
            .iter()
            .map(|store| api_leaf(&store.id, &store.display_name, "artwork-store", &store.state, "/api/storage/artwork", serde_json::json!(store)))
            .collect(),
    }
}

fn api_telemetry_node() -> ApiObjectNode {
    let cpu_temp = cpu_temperature_celsius();
    let cpu_usage = cpu_usage_percent();
    let load = load_average();
    let disk_io = disk_io_counters();
    let io_pressure = pressure_avg10_percent("/proc/pressure/io");
    ApiObjectNode {
        id: "telemetry".to_string(),
        kind: "object".to_string(),
        title: "Telemetry".to_string(),
        state: "observed".to_string(),
        route: Some("/api/root".to_string()),
        summary: serde_json::json!({
            "cpuTemperatureCelsius": cpu_temp,
            "cpuUsagePercent": cpu_usage,
            "loadAverage": load,
            "diskIo": disk_io,
            "ioPressureAvg10": io_pressure,
        }),
        metrics: vec![
            api_metric_value("cpuTemperatureCelsius", "CPU temperature", serde_json::json!(cpu_temp), Some("celsius"), None),
            api_metric_value("cpuUsagePercent", "CPU usage", serde_json::json!(cpu_usage), Some("percent"), None),
            api_metric_value("load1", "Load average 1m", load.get("oneMinute").cloned().unwrap_or(serde_json::Value::Null), None, None),
            api_metric_value("diskReadRate", "Disk read rate", disk_io.get("readBytesPerSec").cloned().unwrap_or(serde_json::Value::Null), Some("bytes_per_second"), None),
            api_metric_value("diskWriteRate", "Disk write rate", disk_io.get("writeBytesPerSec").cloned().unwrap_or(serde_json::Value::Null), Some("bytes_per_second"), None),
            api_metric_value("ioPressureAvg10", "I/O pressure", serde_json::json!(io_pressure), Some("percent"), None),
        ],
        data: serde_json::json!({
            "cpu": { "temperatureCelsius": cpu_temp, "usagePercent": cpu_usage },
            "load": load,
            "io": { "disk": disk_io, "pressureAvg10": io_pressure },
        }),
        children: vec![
            api_leaf("cpu", "CPU", "telemetry", "observed", "/api/root", serde_json::json!({"temperatureCelsius": cpu_temp, "usagePercent": cpu_usage})),
            api_leaf("load", "Load", "telemetry", "observed", "/api/root", load),
            api_leaf("io", "I/O", "telemetry", "observed", "/api/root", disk_io),
        ],
    }
}

fn api_routes_node() -> ApiObjectNode {
    ApiObjectNode {
        id: "routes".to_string(),
        kind: "index".to_string(),
        title: "API Routes".to_string(),
        state: "available".to_string(),
        route: Some("/api/root".to_string()),
        summary: serde_json::json!({"root":"/api/root","legacyStatus":"/api/status","storage":"/api/storage/state"}),
        metrics: Vec::new(),
        data: serde_json::json!({
            "root": "/api/root",
            "status": "/api/status",
            "storage": "/api/storage/state",
            "games": "/api/storage/games",
            "gamescope": "/api/storage/gamescope",
            "artwork": "/api/storage/artwork",
            "network": "/api/network/state",
            "ai": "/api/ai/state",
            "controllers": "/api/controllers/state",
            "system": "/api/system/status"
        }),
        children: Vec::new(),
    }
}

fn api_leaf(
    id: &str,
    title: &str,
    kind: &str,
    state: &str,
    route: &str,
    data: serde_json::Value,
) -> ApiObjectNode {
    ApiObjectNode {
        id: id.to_string(),
        kind: kind.to_string(),
        title: title.to_string(),
        state: state.to_string(),
        route: Some(route.to_string()),
        summary: serde_json::Value::Null,
        metrics: Vec::new(),
        data,
        children: Vec::new(),
    }
}

fn api_metric<T: Serialize>(
    id: &str,
    label: &str,
    value: T,
    unit: Option<&str>,
    state: Option<&str>,
) -> ApiMetric {
    api_metric_value(id, label, serde_json::json!(value), unit, state)
}

fn api_metric_value(
    id: &str,
    label: &str,
    value: serde_json::Value,
    unit: Option<&str>,
    state: Option<&str>,
) -> ApiMetric {
    ApiMetric {
        id: id.to_string(),
        label: label.to_string(),
        value,
        unit: unit.map(str::to_string),
        state: state.map(str::to_string),
    }
}

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn now_unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

fn thermal_zone_priority(zone_type: &str) -> i32 {
    match zone_type {
        "x86_pkg_temp" => 100,
        "tctl" => 95,
        "k10temp" => 90,
        "cpu-thermal" => 85,
        "cpu" => 80,
        "acpitz" => 10,
        other if other.contains("pkg") => 70,
        other if other.contains("cpu") => 60,
        _ => -1,
    }
}

fn cpu_temperature_celsius() -> Option<f64> {
    let thermal_root = Path::new("/sys/class/thermal");
    let entries = fs::read_dir(thermal_root).ok()?;
    let mut best: Option<(i32, f64)> = None;
    for entry in entries.flatten() {
        let zone_path = entry.path();
        let zone_type = fs::read_to_string(zone_path.join("type"))
            .ok()
            .map(|value| value.trim().to_string())
            .unwrap_or_default();
        let priority = thermal_zone_priority(&zone_type);
        if priority < 0 {
            continue;
        }
        let Ok(raw) = fs::read_to_string(zone_path.join("temp")) else {
            continue;
        };
        let Ok(milli_celsius) = raw.trim().parse::<f64>() else {
            continue;
        };
        if milli_celsius <= 0.0 {
            continue;
        }
        let celsius = (milli_celsius / 1000.0 * 10.0).round() / 10.0;
        match best {
            Some((best_priority, _)) if priority < best_priority => continue,
            Some((best_priority, best_temp)) if priority == best_priority && celsius <= best_temp => {
                continue
            }
            _ => best = Some((priority, celsius)),
        }
    }
    best.map(|(_, celsius)| celsius)
}

fn proc_cpu_jiffies() -> Option<(u64, u64)> {
    let raw = fs::read_to_string("/proc/stat").ok()?;
    let line = raw.lines().next()?;
    if !line.starts_with("cpu ") {
        return None;
    }
    let parts = line.split_whitespace().collect::<Vec<_>>();
    if parts.len() < 5 {
        return None;
    }
    let parse = |index: usize| parts.get(index).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
    let user = parse(1);
    let nice = parse(2);
    let system = parse(3);
    let idle = parse(4);
    let iowait = parse(5);
    let irq = parse(6);
    let softirq = parse(7);
    let steal = parse(8);
    let busy = user + nice + system + irq + softirq + steal;
    let total = busy + idle + iowait;
    Some((busy, total))
}

fn cpu_usage_percent() -> Option<f64> {
    let (busy, total) = proc_cpu_jiffies()?;
    let store = LAST_CPU_USAGE_SAMPLE.get_or_init(|| Mutex::new(None));
    let Ok(mut slot) = store.lock() else {
        return None;
    };
    let usage = slot.as_ref().and_then(|previous| {
        let busy_delta = busy.saturating_sub(previous.busy_jiffies);
        let total_delta = total.saturating_sub(previous.total_jiffies);
        if total_delta == 0 {
            return None;
        }
        let percent = (busy_delta as f64 / total_delta as f64) * 100.0;
        Some((percent * 10.0).round() / 10.0)
    });
    *slot = Some(CpuUsageSample {
        busy_jiffies: busy,
        total_jiffies: total,
    });
    usage
}

fn load_average() -> serde_json::Value {
    let Some(raw) = fs::read_to_string("/proc/loadavg").ok() else {
        return serde_json::json!({"available": false});
    };
    let parts = raw.split_whitespace().collect::<Vec<_>>();
    serde_json::json!({
        "available": true,
        "oneMinute": parts.get(0).and_then(|v| v.parse::<f64>().ok()),
        "fiveMinute": parts.get(1).and_then(|v| v.parse::<f64>().ok()),
        "fifteenMinute": parts.get(2).and_then(|v| v.parse::<f64>().ok()),
        "runningProcesses": parts.get(3).copied(),
        "lastPid": parts.get(4).and_then(|v| v.parse::<u64>().ok()),
    })
}

fn pressure_avg10_percent(path: &str) -> Option<f64> {
    let raw = fs::read_to_string(path).ok()?;
    raw.lines().find_map(|line| {
        if !line.starts_with("some ") {
            return None;
        }
        line.split_whitespace().find_map(|part| {
            part.strip_prefix("avg10=")
                .and_then(|value| value.parse::<f64>().ok())
        })
    })
}

fn disk_device_counts_io(name: &str) -> bool {
    if name.starts_with("loop") || name.starts_with("ram") || name.starts_with("dm-") {
        return false;
    }
    if name.starts_with("nvme") {
        return !name.contains('p');
    }
    if name.starts_with("mmcblk") {
        return !name.contains('p');
    }
    if name.starts_with("sd") || name.starts_with("vd") {
        return name.len() == 3;
    }
    false
}

fn disk_io_counters() -> serde_json::Value {
    let Some(raw) = fs::read_to_string("/proc/diskstats").ok() else {
        return serde_json::json!({"available": false});
    };
    let mut reads_completed = 0u64;
    let mut sectors_read = 0u64;
    let mut writes_completed = 0u64;
    let mut sectors_written = 0u64;
    let mut devices = 0u64;
    for line in raw.lines() {
        let parts = line.split_whitespace().collect::<Vec<_>>();
        if parts.len() < 14 {
            continue;
        }
        let name = parts[2];
        if !disk_device_counts_io(name) {
            continue;
        }
        devices += 1;
        reads_completed += parts[3].parse::<u64>().unwrap_or(0);
        sectors_read += parts[5].parse::<u64>().unwrap_or(0);
        writes_completed += parts[7].parse::<u64>().unwrap_or(0);
        sectors_written += parts[9].parse::<u64>().unwrap_or(0);
    }
    let sampled_at_millis = now_unix_millis();
    let (read_bytes_per_sec, write_bytes_per_sec) = {
        let store = LAST_DISK_IO_SAMPLE.get_or_init(|| Mutex::new(None));
        let Ok(mut slot) = store.lock() else {
            return serde_json::json!({
                "available": true,
                "devices": devices,
                "readsCompleted": reads_completed,
                "writesCompleted": writes_completed,
                "sectorsRead": sectors_read,
                "sectorsWritten": sectors_written,
                "readBytesPerSec": 0,
                "writeBytesPerSec": 0,
            });
        };
        let rates = slot.as_ref().map(|previous| {
            let elapsed_ms = sampled_at_millis
                .saturating_sub(previous.sampled_at_millis)
                .max(1);
            let read_delta = sectors_read.saturating_sub(previous.sectors_read);
            let write_delta = sectors_written.saturating_sub(previous.sectors_written);
            (
                read_delta.saturating_mul(512).saturating_mul(1000) / elapsed_ms as u64,
                write_delta
                    .saturating_mul(512)
                    .saturating_mul(1000)
                    / elapsed_ms as u64,
            )
        }).unwrap_or((0, 0));
        *slot = Some(DiskIoSample {
            sectors_read,
            sectors_written,
            sampled_at_millis,
        });
        rates
    };
    serde_json::Value::from(serde_json::json!({
        "available": true,
        "devices": devices,
        "readsCompleted": reads_completed,
        "writesCompleted": writes_completed,
        "sectorsRead": sectors_read,
        "sectorsWritten": sectors_written,
        "readBytesPerSec": read_bytes_per_sec,
        "writeBytesPerSec": write_bytes_per_sec,
    }))
}
