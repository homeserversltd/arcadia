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

async fn api_root_route(State(state): State<Arc<AppState>>) -> Json<ApiRootObject> {
    Json(api_root_object(&state))
}

fn api_root_object(state: &AppState) -> ApiRootObject {
    let status = console_status(state);
    ApiRootObject {
        schema: "arcadia.api.root.v1",
        kind: "arcadia-root",
        id: "arcadia",
        generated_at_unix: now_unix_seconds(),
        product: status.product.clone(),
        canonical_url: status.canonical_url.clone(),
        children: vec![
            api_appliance_node(&status),
            api_storage_node(&status),
            api_telemetry_node(),
            api_routes_node(),
        ],
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
    let load = load_average();
    let disk_io = disk_io_counters();
    ApiObjectNode {
        id: "telemetry".to_string(),
        kind: "object".to_string(),
        title: "Telemetry".to_string(),
        state: "observed".to_string(),
        route: Some("/api/root".to_string()),
        summary: serde_json::json!({
            "cpuTemperatureCelsius": cpu_temp,
            "loadAverage": load,
            "diskIo": disk_io,
        }),
        metrics: vec![
            api_metric_value("cpuTemperatureCelsius", "CPU temperature", serde_json::json!(cpu_temp), Some("celsius"), None),
            api_metric_value("load1", "Load average 1m", load.get("oneMinute").cloned().unwrap_or(serde_json::Value::Null), None, None),
            api_metric_value("diskReads", "Disk reads", disk_io.get("readsCompleted").cloned().unwrap_or(serde_json::Value::Null), Some("count"), None),
            api_metric_value("diskWrites", "Disk writes", disk_io.get("writesCompleted").cloned().unwrap_or(serde_json::Value::Null), Some("count"), None),
        ],
        data: serde_json::json!({
            "cpu": { "temperatureCelsius": cpu_temp },
            "load": load,
            "io": { "disk": disk_io },
        }),
        children: vec![
            api_leaf("cpu", "CPU", "telemetry", "observed", "/api/root", serde_json::json!({"temperatureCelsius": cpu_temp})),
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

fn cpu_temperature_celsius() -> Option<f64> {
    let thermal_root = Path::new("/sys/class/thermal");
    let entries = fs::read_dir(thermal_root).ok()?;
    for entry in entries.flatten() {
        let path = entry.path().join("temp");
        let Ok(raw) = fs::read_to_string(path) else {
            continue;
        };
        if let Ok(milli_celsius) = raw.trim().parse::<f64>() {
            if milli_celsius > 0.0 {
                return Some((milli_celsius / 1000.0 * 10.0).round() / 10.0);
            }
        }
    }
    None
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
        if name.starts_with("loop") || name.starts_with("ram") {
            continue;
        }
        devices += 1;
        reads_completed += parts[3].parse::<u64>().unwrap_or(0);
        sectors_read += parts[5].parse::<u64>().unwrap_or(0);
        writes_completed += parts[7].parse::<u64>().unwrap_or(0);
        sectors_written += parts[9].parse::<u64>().unwrap_or(0);
    }
    serde_json::json!({
        "available": true,
        "devices": devices,
        "readsCompleted": reads_completed,
        "writesCompleted": writes_completed,
        "sectorsRead": sectors_read,
        "sectorsWritten": sectors_written,
        "readBytesApprox": sectors_read.saturating_mul(512),
        "writtenBytesApprox": sectors_written.saturating_mul(512),
    })
}
