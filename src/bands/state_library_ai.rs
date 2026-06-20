
#[derive(Clone, Serialize)]
pub struct LibraryStatus {
    pub detected_games: u64,
    pub detected_files: u64,
    pub gamescope_entries: u64,
    pub first_sync_completed: bool,
    pub last_sync: String,
    pub last_sync_at: Option<String>,
    pub last_sync_state: String,
    pub artwork_status: String,
    pub artwork_complete: u64,
    pub artwork_missing: u64,
    pub sync_needed: bool,
    pub sync_state: String,
    pub unsynced_added: u64,
    pub unsynced_changed: u64,
    pub unsynced_removed: u64,
    pub total_detected_games: u64,
    pub total_synced_entries: u64,
}

#[derive(Clone)]
struct GameFileState {
    normalized_rom_path: String,
    size_bytes: u64,
    mtime_ms: u64,
}

#[derive(Deserialize)]
struct SyncManifestDoc {
    #[serde(default)]
    entries: Vec<SyncManifestEntry>,
}

#[derive(Deserialize)]
struct SyncManifestEntry {
    #[serde(rename = "romPath")]
    rom_path: Option<String>,
    #[serde(rename = "normalizedRomPath")]
    normalized_rom_path: Option<String>,
    #[serde(rename = "sizeBytes", default)]
    size_bytes: u64,
    #[serde(rename = "mtimeMs", default)]
    mtime_ms: u64,
    #[serde(rename = "gamescopeEntryId")]
    gamescope_entry_id: Option<String>,
    #[serde(rename = "lastSyncedAt")]
    last_synced_at: Option<String>,
    #[serde(rename = "artworkStatus")]
    artwork_status: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct LocalAiStatus {
    pub load_state: String,
    pub selected_model_id: Option<String>,
    pub selected_model_name: Option<String>,
    pub loaded_model_id: Option<String>,
    pub loaded_model_name: Option<String>,
    pub loaded_model: Option<String>,
    pub available_models: Vec<LocalAiModelStatus>,
    pub gpu_memory: Option<String>,
    pub gpu_memory_used_bytes: Option<u64>,
    pub gpu_memory_total_bytes: Option<u64>,
    pub lan_inference_enabled: bool,
    pub lan_inference_port: Option<u16>,
}

#[derive(Clone, Serialize)]
pub struct LocalAiModelStatus {
    pub id: String,
    pub name: String,
    pub filename: String,
    pub source: String,
    pub repo_id: Option<String>,
    pub size_bytes: u64,
    pub size: String,
    pub quantization: Option<String>,
    pub estimated_vram_bytes: Option<u64>,
    pub recommended_use: Option<&'static str>,
    pub installed_at: Option<String>,
    pub is_recommended: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAIState {
    pub runtime: AIRuntimeState,
    pub loaded_model: AILoadedModelState,
    pub installed_models: Vec<LocalAiModelStatus>,
    pub recommended_models: Vec<AIRecommendedModel>,
    pub downloads: Vec<AIDownloadState>,
    pub inference: InferenceState,
    pub hardware: AIHardwareState,
    pub activity: AIActivityState,
    pub settings: AISettingsState,
    pub client_handoff: AIClientHandoffState,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AISettingsState {
    pub start_api_on_boot: bool,
    pub auto_load_last_model: bool,
    pub preferred_model_id: Option<String>,
    pub context_size: u32,
    pub gpu_layers: i32,
    pub threads: u32,
    pub batch: u32,
    pub concurrency: u32,
    pub request_limit: u32,
    pub lan_cidr: String,
    pub cors_origins: Vec<String>,
    pub log_level: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AIClientHandoffState {
    pub endpoint: Option<String>,
    pub openai_base_url: Option<String>,
    pub token_configured: bool,
    pub token_preview: Option<String>,
    pub hermes_hint: String,
    pub pi_hint: String,
    pub secret_values_recorded: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AIRuntimeState {
    pub installed: bool,
    pub name: String,
    pub version: Option<String>,
    pub latest_version: Option<String>,
    pub update_state: String,
    pub server_state: String,
    pub binary_path: Option<String>,
    pub source_path: Option<String>,
    pub description: String,
    pub last_checked_at: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AILoadedModelState {
    pub load_state: String,
    pub selected_model_id: Option<String>,
    pub selected_model_name: Option<String>,
    pub loaded_model_id: Option<String>,
    pub loaded_model_name: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AIRecommendedModel {
    pub id: String,
    pub name: String,
    pub description: String,
    pub source: String,
    pub repo_id: Option<String>,
    pub filename: Option<String>,
    pub size_bytes: Option<u64>,
    pub estimated_vram_bytes: Option<u64>,
    pub recommended_use: Option<String>,
    pub install_state: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AIDownloadState {
    pub id: String,
    pub model_name: String,
    pub filename: String,
    pub state: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub speed_bytes_per_second: Option<u64>,
    pub eta_seconds: Option<u64>,
    pub error: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InferenceState {
    pub enabled: bool,
    pub lan_access_enabled: bool,
    pub access_mode: String,
    pub host: String,
    pub port: u16,
    pub endpoint_urls: Vec<String>,
    pub api_mode: Option<String>,
    pub request_count: Option<u64>,
    pub last_request_at: Option<String>,
    pub nginx_configured: bool,
    pub firewall_configured: bool,
    pub health_path: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AIHardwareState {
    pub gpu_memory_used_bytes: Option<u64>,
    pub gpu_memory_total_bytes: Option<u64>,
    pub model_storage_bytes: u64,
    pub free_storage_bytes: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AIActivityState {
    pub current_operation: String,
    pub last_error: Option<String>,
    pub runtime_update_log: String,
    pub model_download_log: String,
    pub model_load_log: String,
    pub inference_server_log: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AIModelIdRequest {
    model_id: Option<String>,
    filename: Option<String>,
    confirm: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AIRecommendedInstallRequest {}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HFListFilesRequest {
    repo_id: String,
    revision: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HFDownloadRequest {
    repo_id: String,
    filename: String,
    revision: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InferenceSetRequest {
    enabled: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InferenceLanRequest {
    enabled: bool,
    port: Option<u16>,
    lan_cidr: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AISettingsRequest {
    start_api_on_boot: Option<bool>,
    auto_load_last_model: Option<bool>,
    preferred_model_id: Option<String>,
    context_size: Option<u32>,
    gpu_layers: Option<i32>,
    threads: Option<u32>,
    batch: Option<u32>,
    concurrency: Option<u32>,
    request_limit: Option<u32>,
    lan_port: Option<u16>,
    lan_cidr: Option<String>,
    cors_origins: Option<Vec<String>>,
    log_level: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenActionRequest {
    confirm: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AIActionResponse {
    ok: bool,
    action: &'static str,
    message: String,
    state: LocalAIState,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HFFileListResponse {
    ok: bool,
    message: String,
    files: Vec<HFModelFile>,
    state: LocalAIState,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HFModelFile {
    filename: String,
    size_bytes: Option<u64>,
    estimated_vram_bytes: Option<u64>,
}

