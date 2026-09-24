#[derive(Clone)]
struct ArcadiaLivingState {
    generated_at_unix: u64,
    document_generation: u64,
    model_lanes: Vec<ModelLane>,
    status: ConsoleStatus,
    network: NetworkState,
    ai: LocalAIState,
    controllers: ControllerStatus,
    controller_input: ControllerInputStatus,
    system: SystemAdminStatus,
    storage: StorageStatus,
    root: ApiRootObject,
    telemetry_root: ApiRootObject,
    document: ApiLivingStateDocument,
}

#[derive(Clone, Default)]
struct ApiTelemetryCache {
    current: serde_json::Value,
    history: serde_json::Value,
    history_fetched_at_unix: u64,
    sampled_at: Option<u64>,
    models: Vec<LocalAiModelStatus>,
}

const FAST_FACTS_CADENCE_SECONDS: u64 = 5;

struct ArcadiaLivingMachine {
    snapshot: Mutex<Option<Arc<ArcadiaLivingState>>>,
    last_expensive_scan_unix: AtomicU64,
    refresh_requested: std::sync::atomic::AtomicBool,
}

impl ArcadiaLivingMachine {
    fn new() -> Self {
        Self {
            snapshot: Mutex::new(None),
            last_expensive_scan_unix: AtomicU64::new(0),
            refresh_requested: std::sync::atomic::AtomicBool::new(false),
        }
    }
    fn try_snapshot(&self) -> Option<Arc<ArcadiaLivingState>> {
        self.snapshot
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
    fn is_initialized(&self) -> bool {
        self.try_snapshot().is_some()
    }
    fn snapshot(&self) -> Arc<ArcadiaLivingState> {
        self.try_snapshot()
            .expect("Arcadia living state not initialized")
    }
    fn request_refresh(&self) {
        self.refresh_requested.store(true, Ordering::Release);
    }
    fn refresh_requested(&self) -> bool {
        self.refresh_requested.load(Ordering::Acquire)
    }
    fn fast_facts_refresh_due(&self, has_active_home_viewer: bool) -> bool {
        has_active_home_viewer || self.refresh_requested()
    }
    fn expensive_scan_due(&self) -> bool {
        self.refresh_requested.swap(false, Ordering::AcqRel)
    }
    fn publish(&self, snapshot: Arc<ArcadiaLivingState>, expensive: bool, document_changed: bool) {
        if expensive {
            self.last_expensive_scan_unix.store(now_unix_seconds(), Ordering::Release);
        }
        let mut guard = self.snapshot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut snapshot = (*snapshot).clone();
        if let Some(current) = guard.as_ref() {
            let input = current.controller_input.clone();
            snapshot.controller_input = input.clone();
            snapshot.controllers.live_input = input.clone();
            snapshot.status.controllers.live_input = input.clone();
            snapshot.document.controllers.live_input = input.clone();
            snapshot.document.status.controllers.live_input = input;
            snapshot.document_generation = if document_changed {
                current.document_generation.saturating_add(1)
            } else {
                current.document_generation
            };
        } else if document_changed {
            snapshot.document_generation = snapshot.document_generation.max(1);
        }
        *guard = Some(Arc::new(snapshot));
    }
    fn publish_controller_input(&self, input: ControllerInputStatus) {
        let mut guard = self.snapshot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(current) = guard.as_ref() else { return; };
        if current.controller_input == input { return; }
        let mut next = (**current).clone();
        next.controller_input = input.clone();
        next.controllers.live_input = input.clone();
        next.status.controllers.live_input = input.clone();
        next.document.controllers.live_input = input.clone();
        next.document.status.controllers.live_input = input;
        next.document_generation = current.document_generation.saturating_add(1);
        *guard = Some(Arc::new(next));
    }
}

fn refresh_controller_input(state: &AppState) {
    let snapshot = state.living.snapshot();
    let devices = &snapshot.controllers.devices;
    let active_device = active_connected_device(devices, &snapshot.controllers.active_controller_id)
        .or_else(|| devices.first().cloned());
    state.living.publish_controller_input(
        read_controller_input_fast(active_device.as_ref()),
    );
}

fn refresh_living_state(state: &AppState) {
    let _publisher = API_SNAPSHOT_PUBLISHER.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let expensive = state.living.expensive_scan_due();
    let previous = state
        .living
        .snapshot
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    let network = network_state(state);
    let network_status = network_status_from_state(&network);
    let storage = if expensive {
        storage_status_with_network(&network_status)
    } else {
        previous
            .as_ref()
            .map(|snapshot| snapshot.storage.clone())
            .unwrap_or_else(|| storage_status_with_network(&network_status))
    };
    let library = if expensive {
        library_status(&storage)
    } else {
        previous.as_ref().map(|snapshot| snapshot.status.library.clone())
            .unwrap_or_else(|| library_status(&storage))
    };
    let local_ai_status = local_ai_status();
    api_telemetry_cache_set_models(&local_ai_status.available_models);
    let model_lanes = api_telemetry_cache().lock().ok()
        .and_then(|cache| cache.current.get("model_lanes").cloned())
        .and_then(|value| serde_json::from_value(value).ok())
        .or_else(|| previous.as_ref().map(|snapshot| snapshot.model_lanes.clone()))
        .unwrap_or_default();
    let ai = local_ai_state_from_status(state, &storage, &local_ai_status);
    let controllers = controller_status_machine();
    let controller_input = controllers.live_input.clone();
    let status = console_status_from_parts(
        state, network_status.clone(), storage.clone(), library, local_ai_status, controllers.clone(),
    );
    let system = status.system.clone();
    let generated_at_unix = now_unix_seconds();
    let root = api_root_object_from_status(state, &status, generated_at_unix);
    let telemetry_root = api_root_telemetry_tick_from_root(&root, generated_at_unix);
    let document = api_living_state_document_from_parts(
        status.clone(),
        storage.clone(),
        network.clone(),
        ai.clone(),
        controllers.clone(),
        system.clone(),
        model_lanes.clone(),
        generated_at_unix,
    );
    state.living.publish(
        Arc::new(ArcadiaLivingState {
            generated_at_unix,
            document_generation: 0,
            model_lanes,
            status,
            network,
            ai,
            controllers,
            controller_input,
            system,
            storage,
            root,
            telemetry_root,
            document,
        }),
        expensive,
        true,
    );
}

fn refresh_living_telemetry(state: &AppState) {
    let _publisher = API_SNAPSHOT_PUBLISHER.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let snapshot = state.living.snapshot();
    let generated_at_unix = now_unix_seconds();
    let mut root = (*snapshot).clone();
    root.generated_at_unix = generated_at_unix;
    root.root = api_root_object_from_status(state, &snapshot.status, generated_at_unix);
    root.telemetry_root = api_root_telemetry_tick_from_root(&root.root, generated_at_unix);
    state.living.publish(Arc::new(root), false, false);
}

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
    pub model_lanes: Vec<ModelLane>,
    pub kind: &'static str,
    pub id: &'static str,
    pub generated_at_unix: u64,
    pub status: ConsoleStatus,
    pub home: ApiHomeState,
    pub sync: ApiSyncState,
    pub storage_pane: ApiStoragePaneState,
    pub local_ai_pane: ApiLocalAiPaneState,
    pub network_pane: ApiNetworkPaneState,
    pub updates_pane: ApiUpdatesPaneState,
    pub system_pane: ApiSystemPaneState,
    pub storage: StorageStatus,
    pub storage_summary: StorageStatus,
    pub network: NetworkState,
    pub ai: LocalAIState,
    pub controllers: ControllerStatus,
    pub system: SystemAdminStatus,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiSystemPaneState {
    pub vault_label: &'static str,
    pub vault_copy: &'static str,
    pub pin_label: &'static str,
    pub pin_copy: &'static str,
    pub admin_text: &'static str,
}

fn api_system_pane_state(status: &ConsoleStatus) -> ApiSystemPaneState {
    ApiSystemPaneState {
        vault_label: if status.vault.mounted { "Unlocked" } else { "Locked" },
        vault_copy: if status.vault.auto_decrypt_enabled { "Automatically decrypts when the console starts." } else { "Unlock required after the console starts." },
        pin_label: if status.gui_pin.pin_required { "PIN required" } else { "Open without PIN" },
        pin_copy: if status.gui_pin.pin_required { "PIN required before accessing HomeConsole." } else { "HomeConsole opens without a PIN." },
        // The living-state document has no browser lease; attendance is overlaid by the lease presenter.
        admin_text: "Guest",
    }
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
pub struct ApiNetworkPaneState {
    pub connection: ApiNetworkPaneConnectionState,
    pub wifi: ApiNetworkPaneWifiState,
    pub wired: ApiNetworkPaneWiredState,
    pub addresses: ApiNetworkPaneAddressState,
    pub reachability: ApiNetworkPaneReachabilityState,
    pub services: ApiNetworkPaneServicesState,
    pub diagnostics: ApiNetworkPaneDiagnosticsState,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiNetworkPaneConnectionState {
    pub state: &'static str,
    pub headline: &'static str,
    pub detail: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiNetworkPaneWifiState {
    pub state: &'static str,
    pub status: &'static str,
    pub ssid: String,
    pub signal_label: String,
    pub signal_percent: String,
    pub signal_visible: bool,
    pub adapter_available: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiNetworkPaneWiredState {
    pub state: &'static str,
    pub badge: String,
    pub mode: &'static str,
    pub nameservers: String,
    pub search: String,
    pub gateway: String,
    pub available: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiNetworkPaneAddressState {
    pub console_url: String,
    pub ip: String,
    pub ip_copyable: bool,
    pub ai_ip_port: String,
    pub ai_url_port: String,
    pub ai_copyable: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiNetworkPaneReachabilityState {
    pub console: &'static str,
    pub console_state: &'static str,
    pub ai: &'static str,
    pub ai_state: &'static str,
    pub internet: &'static str,
    pub internet_state: &'static str,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiNetworkPaneServicesState {
    pub web_console: &'static str,
    pub web_console_state: &'static str,
    pub lan_ai: &'static str,
    pub lan_ai_state: &'static str,
    pub ssh: &'static str,
    pub ssh_state: &'static str,
    pub root_ca: &'static str,
    pub root_ca_state: &'static str,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiNetworkPaneDiagnosticsState {
    pub gateway: &'static str,
    pub dns: &'static str,
    pub internet: &'static str,
    pub lan_ai: &'static str,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiLocalAiPaneState {
    pub hero: ApiLocalAiPaneHeroState,
    pub model: ApiLocalAiPaneModelState,
    pub access: ApiLocalAiPaneAccessState,
    pub port: ApiLocalAiPanePortState,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiLocalAiPaneHeroState {
    pub state: String,
    pub state_class: &'static str,
    pub headline: &'static str,
    pub endpoint: String,
    pub endpoint_available: bool,
    pub endpoint_state: &'static str,
    pub next_action: &'static str,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiLocalAiPaneModelState {
    pub state: String,
    pub state_class: &'static str,
    pub selected: String,
    pub serving_now: String,
    pub selected_detail: &'static str,
    pub serving_detail: &'static str,
    pub library_count: String,
    pub library_detail: &'static str,
    pub load_badge: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiLocalAiPaneAccessState {
    pub state: &'static str,
    pub state_class: &'static str,
    pub internal: &'static str,
    pub internal_detail: &'static str,
    pub lan: &'static str,
    pub lan_detail: &'static str,
    pub port: String,
    pub base_url: String,
    pub base_url_detail: &'static str,
    pub listening_badge: &'static str,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiLocalAiPanePortState {
    pub active: String,
    pub active_badge: String,
    pub state_class: &'static str,
}



#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiUpdatesPaneState {
    pub state: String,
    pub state_label: String,
    pub state_class: &'static str,
    pub last_ran: String,
    pub pending_updates: String,
    pub readiness_ratio: String,
    pub module_line: String,
    pub suite: &'static str,
    pub check: &'static str,
    pub modules: Vec<ApiUpdatesModuleState>,
    pub pinned: Vec<ApiUpdatesPinnedGroupState>,
    pub receipts: ApiUpdatesReceiptsState,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiUpdatesModuleState {
    pub id: String,
    pub label: String,
    pub enabled: bool,
    pub state: String,
    pub state_label: &'static str,
    pub state_class: &'static str,
    pub version: String,
    pub receipt: String,
    pub update_allowed: bool,
    pub update_label: &'static str,
    pub update_message: &'static str,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiUpdatesPinnedGroupState {
    pub label: &'static str,
    pub members: Vec<String>,
    pub button_label: &'static str,
    pub message: &'static str,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiUpdatesReceiptsState {
    pub suite: String,
    pub check: String,
    pub module_root: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiHomeState {
    pub priority: ApiHomePriorityState,
    pub storage: ApiHomeStorageState,
    pub network: ApiHomeNetworkState,
    pub updates: ApiHomeUpdatesState,
    pub ai: ApiHomeAiState,
    pub telemetry: serde_json::Value,
    pub warning: ApiHomeWarningState,
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
    pub percent_used: String,
    pub games_size: String,
    pub ai_size: String,
    pub everything_else_size: String,
    pub free_size: String,
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
    pub modules: Vec<ApiHomeUpdateModuleState>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiHomeUpdateModuleState {
    pub id: String,
    pub label: String,
    pub seeking_update: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiHomeAiState {
    pub state: String,
    pub attention: bool,
    pub load: &'static str,
    pub activity: &'static str,
    pub models: Vec<ApiHomeAiModelState>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiHomeAiModelState {
    pub name: String,
    pub state: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiHomeWarningState {
    pub visible: bool,
    pub title: &'static str,
}

const HOME_TELEMETRY_TOPIC: &str = "home.load";
const HOME_TELEMETRY_CADENCE_SECONDS: u64 = 1;
const HOME_TELEMETRY_RENEW_SECONDS: u64 = 10;
const HOME_TELEMETRY_IDLE_TIMEOUT_SECONDS: u64 = 30;

static HOME_TELEMETRY_LEASE_COUNTER: AtomicU64 = AtomicU64::new(1);
static HOME_TELEMETRY_LEASES: OnceLock<Mutex<HashMap<String, HomeTelemetryLease>>> = OnceLock::new();
static API_TELEMETRY_CACHE: OnceLock<Mutex<ApiTelemetryCache>> = OnceLock::new();

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

fn api_telemetry_cache() -> &'static Mutex<ApiTelemetryCache> {
    API_TELEMETRY_CACHE.get_or_init(|| Mutex::new(ApiTelemetryCache::default()))
}

fn api_telemetry_cache_set_models(models: &[LocalAiModelStatus]) {
    if let Ok(mut cache) = api_telemetry_cache().lock() {
        cache.models = models.to_vec();
    }
}

static API_TELEMETRY_COLLECTING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static API_FULL_REFRESHING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static API_SNAPSHOT_PUBLISHER: Mutex<()> = Mutex::new(());

struct RefreshFlagGuard(&'static std::sync::atomic::AtomicBool);
impl Drop for RefreshFlagGuard {
    fn drop(&mut self) { self.0.store(false, Ordering::Release); }
}

fn refresh_api_telemetry_cache() {
    let current = CaduceusAccessClient::default().get_json("/api/v1/appliance/stats").ok();
    let sampled_at = current.as_ref()
        .and_then(|value| value.get("ts"))
        .and_then(serde_json::Value::as_u64);
    if let Ok(mut cache) = api_telemetry_cache().lock() {
        if let Some(current) = current { cache.current = current; cache.sampled_at = sampled_at; }
    }
    refresh_api_telemetry_history();
}

fn refresh_api_telemetry_history() {
    let now = now_unix_seconds();
    let due = api_telemetry_cache().lock().ok().map(|cache|
        cache.history_fetched_at_unix == 0 || now.saturating_sub(cache.history_fetched_at_unix) >= 60
    ).unwrap_or(false);
    if !due { return; }
    if let Ok(history) = CaduceusAccessClient::default().get_json("/api/v1/appliance/stats/history") {
        if let Ok(mut cache) = api_telemetry_cache().lock() {
            cache.history = history;
            cache.history_fetched_at_unix = now;
        }
    }
}

fn telemetry_source_value(temperature: &serde_json::Value, label: &str, key: &str) -> serde_json::Value {
    temperature.get("bySource").and_then(serde_json::Value::as_array)
        .and_then(|sources| sources.iter().find(|source| source.get("label").and_then(serde_json::Value::as_str) == Some(label)))
        .and_then(|source| source.get(key)).cloned().unwrap_or(serde_json::Value::Null)
}

pub(crate) fn api_telemetry_data() -> serde_json::Value {
    let cache = api_telemetry_cache().lock().ok().map(|cache| cache.clone()).unwrap_or_default();
    let current = &cache.current;
    let load = current.get("load").cloned().unwrap_or(serde_json::Value::Null);
    let one = load.get("one").and_then(serde_json::Value::as_f64);
    let cpu_usage = current.get("cpu").and_then(|cpu| cpu.get("usagePercent")).and_then(serde_json::Value::as_f64);
    let memory = current.get("memory").cloned().unwrap_or(serde_json::Value::Null);
    let total = memory.get("MemTotal").or_else(|| memory.get("totalBytes")).and_then(serde_json::Value::as_u64);
    let used = memory.get("usedBytes").and_then(serde_json::Value::as_u64);
    let used_percent = total.zip(used).map(|(total, used)| (used as f64 / total.max(1) as f64 * 100.0).round());
    let temperature = current.get("temperature").cloned().unwrap_or(serde_json::Value::Null);
    let cpu_temperature = telemetry_source_value(&temperature, "cpu", "celsius");
    let storage_temperature = telemetry_source_value(&temperature, "storage", "celsius");
    let throughput = current.get("disk").and_then(|disk| disk.get("throughput")).cloned().unwrap_or(serde_json::Value::Null);
    serde_json::json!({
        "load": { "oneMinute": one, "fiveMinute": load.get("five").and_then(serde_json::Value::as_f64), "fifteenMinute": load.get("fifteen").and_then(serde_json::Value::as_f64) },
        "cpu": { "usagePercent": cpu_usage, "temperatureCelsius": cpu_temperature },
        "memory": { "usedBytes": used, "totalBytes": total, "usedPercent": used_percent },
        "io": { "disk": { "readBytesPerSec": throughput.get("readBytesPerSecond"), "writeBytesPerSec": throughput.get("writeBytesPerSecond") }, "pressureAvg10": current.get("pressure").and_then(|value| value.get("io")).and_then(|value| value.get("someAvg10")).and_then(serde_json::Value::as_f64) },
        "temperature": temperature, "storageTemperatureCelsius": storage_temperature, "fans": current.get("fans"),
        "gpu": current.get("gpu"),
        "sampledAt": cache.sampled_at, "localAi": { "models": cache.models },
    })
}

fn home_telemetry_format(value: serde_json::Value, suffix: &str) -> String {
    value.as_f64().map(|n| format!("{n:.1}{suffix}")).unwrap_or_else(|| "—".to_string())
}

pub(crate) fn api_home_telemetry_data() -> serde_json::Value {
    let data = api_telemetry_data();
    let text = |path: &[&str], suffix: &str| {
        let value = path.iter().fold(data.clone(), |value, key| {
            value.get(*key).cloned().unwrap_or(serde_json::Value::Null)
        });
        serde_json::Value::String(home_telemetry_format(value, suffix))
    };
    let rate = |key: &str| {
        data.get("io")
            .and_then(|value| value.get("disk"))
            .and_then(|value| value.get(key))
            .and_then(serde_json::Value::as_f64)
            .map(telemetry_human_rate)
            .map(serde_json::Value::String)
            .unwrap_or_else(|| serde_json::Value::String("—".to_string()))
    };
    let memory = data.get("memory").cloned().unwrap_or_default();
    let gpu = data.get("gpu").cloned().unwrap_or_default();
    let models = data
        .get("localAi")
        .and_then(|value| value.get("models"))
        .cloned()
        .unwrap_or_default();
    let fans = data
        .get("fans")
        .and_then(serde_json::Value::as_array)
        .map(|fans| {
            fans.iter()
                .map(|fan| {
                    let rpm = fan
                        .get("rpm")
                        .and_then(serde_json::Value::as_f64)
                        .map(|rpm| format!("{rpm:.0} RPM"))
                        .unwrap_or_else(|| "—".to_string());
                    serde_json::json!({ "label": fan.get("label"), "rpm": rpm })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    serde_json::json!({
        "load": data.get("load"),
        "cpu": {
            "temperatureCelsius": text(&["cpu", "temperatureCelsius"], "°C"),
            "usagePercent": text(&["cpu", "usagePercent"], "%")
        },
        "memory": {
            "usedBytes": memory.get("usedBytes").and_then(serde_json::Value::as_u64).map(human_size).unwrap_or_else(|| "—".to_string()),
            "totalBytes": memory.get("totalBytes").and_then(serde_json::Value::as_u64).map(human_size).unwrap_or_else(|| "—".to_string()),
            "usedPercent": text(&["memory", "usedPercent"], "%")
        },
        "io": {
            "disk": { "readBytesPerSec": rate("readBytesPerSec"), "writeBytesPerSec": rate("writeBytesPerSec") },
            "pressureAvg10": "—"
        },
        "temperature": { "storage": text(&["storageTemperatureCelsius"], "°C") },
        "gpu": {
            "utilizationPercent": gpu.get("utilizationPercent").cloned().map(|value| home_telemetry_format(value, "%")).map(serde_json::Value::String).unwrap_or_else(|| serde_json::Value::String("—".to_string())),
            "temperatureCelsius": gpu.get("temperatureCelsius").cloned().map(|value| home_telemetry_format(value, "°C")).map(serde_json::Value::String).unwrap_or_else(|| serde_json::Value::String("—".to_string()))
        },
        "fans": fans,
        "history": data.get("history"),
        "localAi": { "models": models },
    })
}

pub(crate) fn telemetry_human_rate(bytes_per_sec: f64) -> String {
    if bytes_per_sec <= 0.0 { return "0 B/s".to_string(); }
    if bytes_per_sec < 1024.0 { return format!("{bytes_per_sec:.0} B/s"); }
    if bytes_per_sec < 1024.0 * 1024.0 { return format!("{} KB/s", (bytes_per_sec / 1024.0).round()); }
    let mb = bytes_per_sec / 1024.0 / 1024.0;
    if mb >= 10.0 { format!("{mb:.0} MB/s") } else { format!("{mb:.1} MB/s") }
}
pub(crate) fn cpu_temperature_celsius() -> Option<f64> { api_telemetry_data()["cpu"]["temperatureCelsius"].as_f64() }
pub(crate) fn load_average() -> serde_json::Value { api_telemetry_data()["load"].clone() }
pub(crate) fn pressure_avg10_percent(_path: &str) -> Option<f64> { None }
pub(crate) fn disk_io_counters() -> serde_json::Value { api_telemetry_data()["io"]["disk"].clone() }
pub(crate) fn memory_usage() -> serde_json::Value { api_telemetry_data()["memory"].clone() }

fn home_telemetry_has_active_lease() -> bool {
    let now = now_unix_seconds();
    let Ok(mut leases) = home_telemetry_leases().lock() else {
        return false;
    };
    leases.retain(|_, lease| lease.expires_at_unix >= now);
    !leases.is_empty()
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
    Json(state.living_snapshot().root.clone())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GuestTelemetryProjection {
    sampled_at: serde_json::Value,
    load: serde_json::Value,
    cpu: serde_json::Value,
    memory: serde_json::Value,
    io: serde_json::Value,
    temperature: serde_json::Value,
    storage_temperature_celsius: serde_json::Value,
    fans: serde_json::Value,
    gpu: serde_json::Value,
    local_ai: serde_json::Value,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AdminTelemetryProjection {
    #[serde(flatten)]
    guest: GuestTelemetryProjection,
    admin: bool,
    processes: serde_json::Value,
    per_disk_io: serde_json::Value,
    self_telemetry: serde_json::Value,
}

fn guest_telemetry_projection(data: &serde_json::Value) -> GuestTelemetryProjection {
    GuestTelemetryProjection {
        sampled_at: data.get("sampledAt").cloned().unwrap_or(serde_json::Value::Null),
        load: data.get("load").cloned().unwrap_or(serde_json::Value::Null),
        cpu: data.get("cpu").cloned().unwrap_or(serde_json::Value::Null),
        memory: data.get("memory").cloned().unwrap_or(serde_json::Value::Null),
        io: data.get("io").cloned().unwrap_or(serde_json::Value::Null),
        temperature: data.get("temperature").cloned().unwrap_or(serde_json::Value::Null),
        storage_temperature_celsius: data.get("storageTemperatureCelsius").cloned().unwrap_or(serde_json::Value::Null),
        fans: data.get("fans").cloned().unwrap_or(serde_json::Value::Null),
        gpu: data.get("gpu").cloned().unwrap_or(serde_json::Value::Null),
        local_ai: data.get("localAi").cloned().unwrap_or(serde_json::Value::Null),
    }
}

async fn api_root_pull_route(State(state): State<Arc<AppState>>, headers: axum::http::HeaderMap) -> Response {
    let attendance = attendance_from_headers(&headers);
    let document = document_incarnation_from_headers(&headers);
    let admin = if let (Some(document), Some(attendance)) = (document, attendance) {
        let document_for_validation = document.clone();
        tokio::task::spawn_blocking(move || attendance_cached_validate(&document_for_validation, &attendance))
            .await.unwrap_or(false)
    } else { false };
    let mut root = state.living_snapshot().root.clone();
    if let Some(node) = root.children.iter_mut().find(|node| node.id == "telemetry") {
        let data = api_telemetry_data();
        node.data = if admin {
            let current = api_telemetry_cache().lock().ok().map(|cache| cache.current.clone()).unwrap_or_default();
            let projection = AdminTelemetryProjection {
                guest: guest_telemetry_projection(&data),
                admin: true,
                processes: current.get("processes").cloned().unwrap_or(serde_json::Value::Null),
                per_disk_io: current.get("disk").and_then(|disk| disk.get("io")).cloned().unwrap_or(serde_json::Value::Null),
                self_telemetry: current.get("self").cloned().unwrap_or(serde_json::Value::Null),
            };
            serde_json::to_value(projection).unwrap_or_default()
        } else {
            serde_json::to_value(guest_telemetry_projection(&data)).unwrap_or_default()
        };
    }
    Json(root).into_response()
}

async fn api_root_history_route() -> Json<serde_json::Value> {
    let (tier, entries) = api_telemetry_cache().lock().ok().map(|cache| {
        let history = &cache.history;
        let raw = history.get("tiers").and_then(|tiers| tiers.get("raw"))
            .and_then(serde_json::Value::as_array)
            .map(|entries| entries.iter().rev().filter_map(|entry| {
                let ts = entry.get("ts")?.as_u64()?;
                let one = entry.get("load")?.get("one")?.as_f64()?;
                Some(serde_json::json!({"ts": ts, "load": {"one": one}}))
            }).take(60).collect::<Vec<_>>())
            .unwrap_or_default();
        if !raw.is_empty() {
            ("raw", raw.into_iter().rev().collect::<Vec<_>>())
        } else {
            let minute = history.get("tiers").and_then(|tiers| tiers.get("minute"))
                .and_then(serde_json::Value::as_array)
                .map(|entries| entries.iter().rev().filter_map(|entry| {
                    // Caduceus minute buckets are Unix minutes; the page consumes Unix seconds.
                    let ts = entry.get("bucket")?.as_u64()?.checked_mul(60)?;
                    let one = entry.get("aggregation")?.get("loadOne")?.as_f64()?;
                    Some(serde_json::json!({"ts": ts, "load": {"one": one}}))
                }).take(60).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>())
                .unwrap_or_default();
            ("minute", minute)
        }
    }).unwrap_or_else(|| ("raw", Vec::new()));
    Json(serde_json::json!({
        "schema": "arcadia.api.root.history.v1",
        "history": {"tiers": {(tier): entries}}
    }))
}

async fn api_root_state_route(State(state): State<Arc<AppState>>) -> Json<ApiLivingStateDocument> {
    Json(state.living_snapshot().document.clone())
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
        let lease_json = serde_json::to_string(&lease)
            .unwrap_or_else(|_| "{}".to_string());
        yield Ok(Event::default().event("lease").data(lease_json));
        yield Ok(Event::default().event("stats.tick").data("{}"));

        let initial = state.living_snapshot();
        let mut last_document_generation = initial.document_generation;
        yield Ok(Event::default().event("state.changed").data("{}"));

        let mut tick = tokio::time::interval(Duration::from_secs(HOME_TELEMETRY_CADENCE_SECONDS));
        tick.tick().await;
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

            yield Ok(Event::default().event("stats.tick").data("{}"));
            let snapshot = state.living_snapshot();
            let document_generation = snapshot.document_generation;
            if last_document_generation != document_generation {
                yield Ok(Event::default().event("state.changed").data("{}"));
                last_document_generation = document_generation;
            }

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

#[cfg(test)]
fn api_root_object(state: &AppState) -> ApiRootObject {
    let status = console_status(state);
    api_root_object_from_status(state, &status, now_unix_seconds())
}

fn api_root_object_from_status(_state: &AppState, status: &ConsoleStatus, generated_at_unix: u64) -> ApiRootObject {
    ApiRootObject {
        schema: "arcadia.api.root.v1",
        kind: "arcadia-root",
        id: "arcadia",
        generated_at_unix,
        product: status.product.clone(),
        canonical_url: status.canonical_url.clone(),
        children: vec![
            api_appliance_node(status),
            api_storage_node(status),
            api_updates_node(status),
            api_telemetry_node(),
            api_routes_node(),
        ],
    }
}

#[cfg(test)]
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

#[cfg(test)]
fn api_living_state_document(state: &AppState) -> ApiLivingStateDocument {
    let status = console_status(state);
    let storage = storage_status();
    ApiLivingStateDocument {
        schema: "arcadia.api.state.v1",
        model_lanes: Vec::new(),
        kind: "arcadiaLivingState",
        id: "arcadia-state",
        generated_at_unix: now_unix_seconds(),
        home: api_home_state(&status),
        sync: api_sync_state(&status),
        storage_pane: api_storage_pane_state(&status),
        local_ai_pane: api_local_ai_pane_state(&status),
        network_pane: api_network_pane_state(&status),
        updates_pane: api_updates_pane_state(&status),
        system_pane: api_system_pane_state(&status),
        status,
        storage: storage.clone(),
        storage_summary: storage,
        network: network_state(state),
        ai: local_ai_state(state),
        controllers: controller_status_api(),
        system: system_admin_status(&network_status(), &hostname()),
    }
}


fn api_updates_state_label(state: &str) -> String {
    match state {
        "current" => "Current".to_string(),
        "available" => "Updates available".to_string(),
        "repair_pending" => "Update needed".to_string(),
        "unknown" => "Check needed".to_string(),
        other => other.split(['-', '_']).filter(|part| !part.is_empty()).map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        }).collect::<Vec<_>>().join(" "),
    }
}

fn api_updates_state_class(state: &str) -> &'static str {
    match state {
        "current" => "available",
        "available" | "repair_pending" => "error",
        "unknown" => "unknown",
        _ => "disabled",
    }
}

fn api_updates_module_state(module: &crate::HarmoniaModuleStatus) -> (&'static str, &'static str) {
    if !module.enabled {
        ("Disabled", "disabled")
    } else if module.present {
        ("Enabled", "available")
    } else {
        ("Update needed", "error")
    }
}

fn api_updates_pane_state(status: &ConsoleStatus) -> ApiUpdatesPaneState {
    let enabled = status.updates.modules.iter().filter(|module| module.enabled).count();
    let ready = status.updates.modules.iter().filter(|module| module.enabled && module.present).count();
    ApiUpdatesPaneState {
        state: status.updates.state.clone(),
        state_label: api_updates_state_label(&status.updates.state),
        state_class: api_updates_state_class(&status.updates.state),
        last_ran: status.updates.last_update_run.clone(),
        pending_updates: status.updates.pending_updates.to_string(),
        readiness_ratio: format!("{ready}/{enabled}"),
        module_line: format!("{} ready · {} need update", ready, enabled.saturating_sub(ready)),
        suite: if status.updates.suite_ok { "Current" } else { "Needs update" },
        check: if status.updates.check_ok { "Current" } else { "Check needed" },
        modules: status.updates.modules.iter()
            .filter(|module| module.pinned_module_membership.is_none() || module.pinned_module_membership.as_deref() == Some("unpinned"))
            .map(|module| {
            let (state_label, state_class) = api_updates_module_state(module);
            let update_allowed = module.pinned_module_membership.as_deref() == Some("unpinned");
            ApiUpdatesModuleState {
                id: module.id.clone(),
                label: module.label.clone(),
                enabled: module.enabled,
                state: module.state.clone(),
                state_label,
                state_class,
                version: if module.present { status.updates.current_version.clone() } else { "Pending".to_string() },
                receipt: module.receipt_path.clone(),
                update_allowed,
                update_label: if update_allowed { "Update module" } else { "Independent update unavailable" },
                update_message: if update_allowed { "Ready for an independent update." } else { "Harmonia membership does not permit an independent update." },
            }
        }).collect(),
        pinned: {
            let members = status.updates.modules.iter()
                .filter(|module| module
                    .pinned_module_membership
                    .as_deref()
                    .is_some_and(|membership| membership != "unpinned"))
                .map(|module| module.label.clone())
                .collect::<Vec<_>>();
            if members.is_empty() {
                Vec::new()
            } else {
                vec![ApiUpdatesPinnedGroupState {
                    label: "Pinned modules",
                    members,
                    button_label: "Update pinned modules",
                    message: "Updates the pinned module group with the whole-suite Sync action.",
                }]
            }
        },
        receipts: ApiUpdatesReceiptsState {
            suite: status.updates.latest_receipt.clone(),
            check: status.updates.latest_check_receipt.clone(),
            module_root: status.updates.module_root.clone(),
        },
    }
}


fn api_network_wifi_status_label(status: &ConsoleStatus) -> &'static str {
    if !status.network.wifi_adapter_available {
        "Unavailable"
    } else if status.network.active_type == "wifi" {
        "On"
    } else {
        "Available"
    }
}

fn api_network_ethernet_head_badge(status: &ConsoleStatus) -> String {
    if !status.network.ethernet_connected {
        return "No cable".to_string();
    }
    status
        .network
        .ethernet_speed_mbps
        .map(|v| format!("{v} Mbps link"))
        .unwrap_or_else(|| "Link up".to_string())
}

fn api_network_ssh_service_label(state: &str) -> &'static str {
    match state {
        "running" | "available" | "active" => "Available",
        _ => "Disabled",
    }
}

fn api_network_pane_state(status: &ConsoleStatus) -> ApiNetworkPaneState {
    let ip_copyable = status.network.ip_address != "—";
    let lan_ai_port = status.local_ai.lan_inference_port.unwrap_or(7777);
    let ai_copyable = ip_copyable && status.network.lan_ai_reachable;
    let ai_ip_port = if ip_copyable {
        format!("{}:{}", status.network.ip_address, lan_ai_port)
    } else {
        "Unavailable".to_string()
    };
    let ai_url_port = if ip_copyable {
        format!("http://{}:{}", status.network.ip_address, lan_ai_port)
    } else {
        "Unavailable".to_string()
    };
    let internet_ok = matches!(status.network.internet_reachable, Some(true));
    ApiNetworkPaneState {
        connection: ApiNetworkPaneConnectionState {
            state: if status.network.online { "available" } else { "disabled" },
            headline: if status.network.online { "Online" } else { "Offline" },
            detail: status.network.connection_session_detail.clone(),
        },
        wifi: ApiNetworkPaneWifiState {
            state: if !status.network.wifi_adapter_available { "disabled" } else if status.network.active_type == "wifi" { "available" } else { "unknown" },
            status: api_network_wifi_status_label(status),
            ssid: if status.network.active_type == "wifi" {
                status.network.ssid.clone().unwrap_or_else(|| "Wi-Fi".to_string())
            } else {
                "Ready for wireless setup".to_string()
            },
            signal_label: status.network.signal_percent.map(|v| format!("{v}% signal")).unwrap_or_else(|| "Signal unavailable".to_string()),
            signal_percent: format!("{}%", status.network.signal_percent.unwrap_or(0)),
            signal_visible: status.network.active_type == "wifi",
            adapter_available: status.network.wifi_adapter_available,
        },
        wired: ApiNetworkPaneWiredState {
            state: if status.network.ethernet_connected { "available" } else { "disabled" },
            badge: api_network_ethernet_head_badge(status),
            mode: if status.network.ethernet_dhcp { "DHCP" } else { "Manual" },
            nameservers: status.network.resolv_nameservers.clone(),
            search: status.network.resolv_search.clone(),
            gateway: status.network.gateway.clone().unwrap_or_else(|| "Unknown".to_string()),
            available: status.network.ethernet_available,
        },
        addresses: ApiNetworkPaneAddressState {
            console_url: status.identity.web_origin.clone(),
            ip: status.network.ip_address.clone(),
            ip_copyable,
            ai_ip_port,
            ai_url_port,
            ai_copyable,
        },
        reachability: ApiNetworkPaneReachabilityState {
            console: if status.network.console_reachable { "Reachable" } else { "Offline" },
            console_state: if status.network.console_reachable { "available" } else { "disabled" },
            ai: if status.network.lan_ai_reachable { "Available" } else { "Disabled" },
            ai_state: if status.network.lan_ai_reachable { "available" } else { "disabled" },
            internet: if internet_ok { "Online" } else { "Offline" },
            internet_state: if internet_ok { "available" } else { "disabled" },
        },
        services: ApiNetworkPaneServicesState {
            web_console: "Available",
            web_console_state: "available",
            lan_ai: if status.network.lan_ai_reachable { "Available" } else { "Disabled" },
            lan_ai_state: if status.network.lan_ai_reachable { "available" } else { "disabled" },
            ssh: api_network_ssh_service_label(&status.system.ssh.service_state),
            ssh_state: if api_network_ssh_service_label(&status.system.ssh.service_state) == "Available" { "available" } else { "disabled" },
            root_ca: if status.system.trust.ca_installed { "Installed" } else { "Needed" },
            root_ca_state: if status.system.trust.ca_installed { "available" } else { "unknown" },
        },
        diagnostics: ApiNetworkPaneDiagnosticsState {
            gateway: "Ready",
            dns: "Ready",
            internet: "Ready",
            lan_ai: "Ready",
        },
    }
}

fn api_local_ai_pane_state(status: &ConsoleStatus) -> ApiLocalAiPaneState {
    let selected_name = status
        .local_ai
        .selected_model_name
        .clone()
        .unwrap_or_else(|| "No model selected".to_string());
    let loaded_name = status
        .local_ai
        .loaded_model_name
        .clone()
        .unwrap_or_else(|| "No model loaded".to_string());
    let port = status.local_ai.lan_inference_port.unwrap_or(7777);
    let endpoint = format!(
        "{}:{}",
        status.identity.web_origin.trim_end_matches('/'),
        port
    );
    let base_url = format!("{endpoint}/v1");
    let api_ready = status.local_ai.lan_inference_enabled;
    let model_count = status.local_ai.available_models.len();
    let library_model_count = status.local_ai.library_models.len();
    let selected_present = status.local_ai.selected_model_id.is_some();
    let model_loaded = matches!(
        status.local_ai.load_state.as_str(),
        "hot" | "loaded" | "running"
    ) || status.local_ai.loaded_model_id.is_some()
        || status.local_ai.loaded_model_name.is_some();
    let state = api_local_ai_model_state_label(status, model_loaded, selected_present, model_count);
    let state_class = api_local_ai_state_class(&status.local_ai.load_state, model_loaded, model_count);
    let endpoint_available = api_ready;
    ApiLocalAiPaneState {
        hero: ApiLocalAiPaneHeroState {
            state: state.to_string(),
            state_class,
            headline: if model_loaded { "Model loaded and serving" } else { "No model serving" },
            endpoint: if endpoint_available { endpoint.clone() } else { "No active endpoint".to_string() },
            endpoint_available,
            endpoint_state: if endpoint_available { "available" } else { "disabled" },
            next_action: if model_count == 0 {
                "Import a GGUF model"
            } else if !selected_present {
                "Select a model"
            } else if !model_loaded {
                "Load the selected model"
            } else if !api_ready {
                "Turn API on"
            } else {
                "Copy the client endpoint"
            },
        },
        model: ApiLocalAiPaneModelState {
            state: state.to_string(),
            state_class,
            selected: selected_name,
            serving_now: loaded_name,
            selected_detail: if selected_present { "Ready to load" } else { "Choose or import a model" },
            serving_detail: if model_loaded { "Available for client calls" } else { "No model invoked" },
            library_count: format!("{library_model_count} library"),
            library_detail: if library_model_count == 0 { "Empty" } else { "Plain list ready" },
            load_badge: status.local_ai.load_state.clone(),
        },
        access: ApiLocalAiPaneAccessState {
            state: if api_ready { "API reachable" } else { "API NOT LISTENING" },
            state_class: if api_ready { "available" } else { "disabled" },
            internal: if api_ready { "Listening" } else { "Off" },
            internal_detail: if api_ready { "Health test can run now" } else { "Load a model before client calls" },
            lan: if api_ready { "Trusted LAN enabled" } else { "Console only" },
            lan_detail: if api_ready { "Trusted LAN only" } else { "Disabled until explicitly enabled" },
            port: port.to_string(),
            base_url: if api_ready { base_url } else { "Unavailable".to_string() },
            base_url_detail: if api_ready { "Use this in clients" } else { "No base URL until API listens" },
            listening_badge: if api_ready { "Listening" } else { "API NOT LISTENING" },
        },
        port: ApiLocalAiPanePortState {
            active: port.to_string(),
            active_badge: format!("Active {port}"),
            state_class: if api_ready { "available" } else { "unknown" },
        },
    }
}

fn api_local_ai_model_state_label(
    status: &ConsoleStatus,
    model_loaded: bool,
    selected_present: bool,
    model_count: usize,
) -> &'static str {
    if status.local_ai.load_state == "error" {
        "Backend error"
    } else if model_loaded {
        "Model loaded"
    } else if selected_present {
        "Model selected, not loaded"
    } else if model_count > 0 {
        "Models installed, none selected"
    } else {
        "No model loaded"
    }
}

fn api_local_ai_state_class(load_state: &str, model_loaded: bool, model_count: usize) -> &'static str {
    if load_state == "error" {
        "error"
    } else if model_loaded {
        "available"
    } else if model_count > 0 {
        "partial"
    } else {
        "disabled"
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

pub(crate) fn api_home_ai_models(status: &ConsoleStatus) -> Vec<ApiHomeAiModelState> {
    let loaded_id = status.local_ai.loaded_model_id.as_deref();
    let mut models = status.local_ai.available_models.iter().map(|model| ApiHomeAiModelState { name: model.name.clone(), state: if loaded_id == Some(model.id.as_str()) { "Loaded".to_string() } else { "Available".to_string() } }).collect::<Vec<_>>();
    models.sort_by(|a, b| b.state.cmp(&a.state).then_with(|| a.name.cmp(&b.name)));
    models
}

fn api_home_state(status: &ConsoleStatus) -> ApiHomeState {
    let priority = api_home_priority_state(status);
    let storage_attention = status.storage.percent_used >= 90;
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
            percent_used: format!("{}% used", status.storage.percent_used),
            games_size: status.storage.games.size.clone(),
            ai_size: status.storage.ai_models.size.clone(),
            everything_else_size: human_size(status.storage.artwork.bytes.saturating_add(status.storage.other.bytes)),
            free_size: status.storage.free.clone(),
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
            modules: enabled_home_update_modules(&status.updates.modules)
                .into_iter()
                .map(|module| ApiHomeUpdateModuleState {
                    id: module.id.clone(),
                    label: module.label.clone(),
                    seeking_update: module.seeking_update,
                })
                .collect(),
        },
        ai: ApiHomeAiState {
            state: status.local_ai.load_state.clone(),
            attention: ai_attention,
            load: api_home_local_ai_load_label(&status.local_ai.load_state),
            activity: api_home_local_ai_activity_label(status),
            models: api_home_ai_models(status),
        },
        telemetry: api_home_telemetry_data(),
        warning: ApiHomeWarningState {
            visible: status.library.last_sync_state == "error" || status.local_ai.load_state == "error",
            title: if status.library.last_sync_state == "error" { "Sync failed" } else { "Local AI error" },
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

fn api_updates_node(status: &ConsoleStatus) -> ApiObjectNode {
    ApiObjectNode {
        id: "updatesPane".to_string(),
        kind: "updatesPane".to_string(),
        title: "Updates".to_string(),
        state: status.updates.state.clone(),
        route: Some("/api/root".to_string()),
        summary: serde_json::json!({
            "state": status.updates.state,
            "lastRan": status.updates.last_update_run,
            "pendingUpdates": status.updates.pending_updates,
            "modules": status.updates.module_count,
        }),
        metrics: vec![
            api_metric("pendingUpdates", "Updates available", status.updates.pending_updates, Some("count"), Some(&status.updates.state)),
            api_metric("modules", "Modules", status.updates.module_count, Some("count"), None),
        ],
        data: serde_json::json!({"updatesPane": api_updates_pane_state(status)}),
        children: status.updates.modules.iter().map(|module| {
            let (_, state_class) = api_updates_module_state(module);
            api_leaf(&module.id, &module.label, "updatesModule", state_class, "/api/root", serde_json::json!(module))
        }).collect(),
    }
}

fn api_telemetry_node() -> ApiObjectNode {
    let data = api_telemetry_data();
    ApiObjectNode {
        id: "telemetry".to_string(), kind: "object".to_string(), title: "Telemetry".to_string(),
        state: "observed".to_string(), route: Some("/api/root".to_string()),
        summary: data.clone(), metrics: vec![
            api_metric_value("cpuTemperatureCelsius", "CPU temperature", data["cpu"]["temperatureCelsius"].clone(), Some("celsius"), None),
            api_metric_value("cpuUsagePercent", "CPU usage", data["cpu"]["usagePercent"].clone(), Some("percent"), None),
            api_metric_value("memoryUsedBytes", "Memory used", data["memory"]["usedBytes"].clone(), Some("bytes"), None),
            api_metric_value("load1", "Load average 1m", data["load"]["oneMinute"].clone(), None, None),
            api_metric_value("diskReadRate", "Disk read rate", data["io"]["disk"]["readBytesPerSec"].clone(), Some("bytes_per_second"), None),
            api_metric_value("diskWriteRate", "Disk write rate", data["io"]["disk"]["writeBytesPerSec"].clone(), Some("bytes_per_second"), None),
            api_metric_value("gpuUtilizationPercent", "GPU utilization", data["gpu"]["utilizationPercent"].clone(), Some("percent"), None),
            api_metric_value("gpuTemperatureCelsius", "GPU temperature", data["gpu"]["temperatureCelsius"].clone(), Some("celsius"), None),
            api_metric_value("fanRpm", "Fan speed", data["fans"].as_array().and_then(|fans| fans.first()).and_then(|fan| fan.get("rpm")).cloned().unwrap_or(serde_json::Value::Null), Some("rpm"), None),
        ], data: data.clone(),
        children: vec![
            api_leaf("cpu", "CPU", "telemetry", "observed", "/api/root", data.get("cpu").cloned().unwrap_or_default()),
            api_leaf("memory", "Memory", "telemetry", "observed", "/api/root", data.get("memory").cloned().unwrap_or_default()),
            api_leaf("load", "Load", "telemetry", "observed", "/api/root", data.get("load").cloned().unwrap_or_default()),
            api_leaf("io", "I/O", "telemetry", "observed", "/api/root", data.get("io").cloned().unwrap_or_default()),
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

fn api_root_telemetry_tick_from_root(
    root: &ApiRootObject,
    generated_at_unix: u64,
) -> ApiRootObject {
    ApiRootObject {
        schema: "arcadia.api.root.v1",
        kind: "arcadia-root",
        id: "arcadia",
        generated_at_unix,
        product: root.product.clone(),
        canonical_url: root.canonical_url.clone(),
        children: vec![api_telemetry_node()],
    }
}


fn api_living_state_document_from_parts(
    status: ConsoleStatus,
    storage: StorageStatus,
    network: NetworkState,
    ai: LocalAIState,
    controllers: ControllerStatus,
    system: SystemAdminStatus,
    model_lanes: Vec<ModelLane>,
    generated_at_unix: u64,
) -> ApiLivingStateDocument {
    ApiLivingStateDocument {
        schema: "arcadia.api.state.v1",
        kind: "arcadiaLivingState",
        id: "arcadia-state",
        generated_at_unix,
        home: api_home_state(&status),
        sync: api_sync_state(&status),
        storage_pane: api_storage_pane_state(&status),
        local_ai_pane: api_local_ai_pane_state(&status),
        network_pane: api_network_pane_state(&status),
        updates_pane: api_updates_pane_state(&status),
        system_pane: api_system_pane_state(&status),
        status,
        model_lanes,
        storage: storage.clone(),
        storage_summary: storage,
        network,
        ai,
        controllers,
        system,
    }
}


#[cfg(test)]
mod deferred_warmup_tests {
    use super::*;

    #[test]
    fn uninitialized_snapshot_is_observable_without_poisoning_lock() {
        let machine = ArcadiaLivingMachine::new();
        assert!(!machine.is_initialized());
        assert!(machine.try_snapshot().is_none());
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| machine.snapshot()));
        assert!(panic.is_err());
        assert!(machine.snapshot.lock().is_ok());
    }

    #[test]
    fn fast_facts_refresh_is_gated_by_viewer_or_explicit_request() {
        let machine = ArcadiaLivingMachine::new();
        assert!(!machine.fast_facts_refresh_due(false));
        assert!(machine.fast_facts_refresh_due(true));

        machine.request_refresh();
        assert!(machine.fast_facts_refresh_due(false));
        assert!(machine.expensive_scan_due());
        assert!(!machine.fast_facts_refresh_due(false));
    }
}
