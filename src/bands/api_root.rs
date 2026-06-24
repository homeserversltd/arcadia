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
