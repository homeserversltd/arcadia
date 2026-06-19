use axum::{
    extract::{Query, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
    collections::HashMap,
    env, fs,
    fs::OpenOptions,
    io::Write,
    net::{Ipv4Addr, SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

mod ui;

const APP_CSS: &str = include_str!("../static/app.css");
const APP_JS: &str = include_str!("../static/app.js");
const GUI_PIN_STATE_PATH: &str = "/var/lib/homeconsole/gui-pin-access.json";
const GUI_PIN_VERIFY_HELPER: &str = "/usr/local/sbin/homeconsole-gui-pin-verify";
const GUI_PIN_ACCESS_HELPER: &str = "/usr/local/sbin/homeconsole-gui-pin-access";
const GUI_PIN_CHANGE_HELPER: &str = "/usr/local/sbin/homeconsole-gui-pin-change";
const GUI_PIN_RESET_HELPER: &str = "/usr/local/sbin/homeconsole-gui-pin-reset-default";
const PROVIDER_KEYS_PATH: &str = "/etc/arch-game-sync/providers.env";
const HARMONIA_BIN: &str = "/usr/local/bin/harmonia";
const HOMECONSOLE_PROFILE: &str = "/etc/harmonia/profiles/homeconsole/index.json";
const ARCH_GAME_SYNC_BIN: &str = "/usr/local/bin/arch-game-sync";
const SYSTEMCTL_BIN: &str = "/usr/bin/systemctl";
const SYSTEMD_RUN_BIN: &str = "/usr/bin/systemd-run";
const GAMES_ROOT: &str = "/home/owner/Games";
const ARTWORK_ROOT: &str = "/home/owner/Games/artwork";
const TEMP_CLEAN_ROOTS: [&str; 2] = ["/tmp", "/var/tmp"];
const MODEL_SCAN_ROOTS: [&str; 3] = ["/home/owner", "/opt", "/var/lib"];
const MODEL_EXTENSIONS: [&str; 3] = ["gguf", "safetensors", "onnx"];
const GAME_SYSTEMS: [&str; 12] = [
    "gba", "genesis", "snes", "nes", "ps1", "n64", "ps2", "sega-cd", "psp", "gamecube", "wii",
    "dos",
];
const SYNC_MANIFEST_PATHS: [&str; 3] = [
    "/var/lib/homeconsole-sync/manifest.json",
    "/var/lib/arch-game-sync/manifest.json",
    "/var/lib/harmonia/state/homeconsole-sync-manifest.json",
];
const LAN_INFERENCE_PORT: u16 = 7777;
const LOCAL_AI_STATE_PATH: &str = "/var/lib/arcadia/local-ai-state.json";
const LOCAL_AI_MODEL_ROOT: &str = "/var/lib/arcadia/models";
const LLAMA_SERVER_BIN: &str = "/usr/local/bin/llama-server";
const LLAMA_CPP_BIN: &str = "/usr/local/bin/llama-cli";
const SAMBA_SERVICE_NAMES: [&str; 2] = ["smb.service", "smbd.service"];
const NETWORK_MANAGER_BIN: &str = "/usr/bin/nmcli";

#[derive(Clone)]
struct AppState {
    started_unix: u64,
    canonical_url: String,
    product: String,
}

#[derive(Clone, Serialize)]
pub struct Health {
    ok: bool,
    service: &'static str,
    product: String,
    version: &'static str,
    started_unix: u64,
}

#[derive(Clone, Serialize)]
pub struct ConsoleStatus {
    pub schema: &'static str,
    pub product: String,
    pub canonical_url: String,
    pub arcadia: ArcadiaStatus,
    pub runtime: RuntimeStatus,
    pub gui_pin: GuiPinStatus,
    pub identity: IdentityStatus,
    pub surfaces: SurfaceStatus,
    pub samba: SambaStatus,
    pub storage: StorageStatus,
    pub network: NetworkStatus,
    pub library: LibraryStatus,
    pub local_ai: LocalAiStatus,
    pub updates: UpdatesStatus,
    pub ui_contract: UiContract,
}

#[derive(Clone, Serialize)]
pub struct IdentityStatus {
    pub product_name: String,
    pub hostname: String,
    pub local_domain: Option<String>,
    pub netbios_name: Option<String>,
    pub web_origin: String,
    pub version: String,
}

#[derive(Clone, Serialize)]
pub struct RuntimeStatus {
    pub machine_uptime: String,
    pub machine_uptime_seconds: u64,
    pub arcadia_uptime: String,
    pub arcadia_uptime_seconds: u64,
}

#[derive(Clone, Serialize)]
pub struct ArcadiaStatus {
    pub service: &'static str,
    pub version: &'static str,
    pub mode: &'static str,
    pub ui: &'static str,
}

#[derive(Clone, Serialize)]
pub struct UiContract {
    pub schema: &'static str,
    pub button_variants: [&'static str; 3],
    pub composition: &'static str,
    pub modal: &'static str,
}

#[derive(Clone, Serialize)]
pub struct GuiPinStatus {
    pub pin_required: bool,
    pub state_path: &'static str,
    pub access_helper_present: bool,
    pub pin_change_helper_present: bool,
    pub pin_reset_helper_present: bool,
    pub pin_storage: &'static str,
    pub default_reset_available: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageStatus {
    pub scanned_at: String,
    pub scanning: bool,
    pub scan_error: Option<String>,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
    pub percent_used: u8,
    pub thresholds: StorageThresholds,
    pub volumes: Vec<StorageVolume>,
    pub registry: StorageRegistry,
    pub categories: StorageCategories,
    pub game_folders: Vec<GameFolderStorage>,
    pub artwork_stores: Vec<FolderStorage>,
    pub ai_model_files: Vec<AIModelStorage>,
    pub cleanup: CleanupState,
    pub diagnostics: StorageDiagnostics,

    // Legacy/home-summary fields consumed by the existing Arcadia shell.
    pub health: &'static str,
    pub header_state: String,
    pub header_class: &'static str,
    pub header_tooltip: String,
    pub ok_copy: &'static str,
    pub warning_copy: &'static str,
    pub total: String,
    pub used: String,
    pub free: String,
    pub percent: String,
    pub games: StorageCategoryStatus,
    pub artwork: StorageCategoryStatus,
    pub ai_models: AiModelStorageStatus,
    pub other: StorageCategoryStatus,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageThresholds {
    pub getting_full_percent: u8,
    pub low_percent: u8,
    pub full_percent: u8,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageVolume {
    pub id: String,
    pub label: String,
    pub mount_point: String,
    pub filesystem: Option<String>,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
    pub health: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageRegistry {
    pub volumes: Vec<StorageVolume>,
    pub categories: StorageRegistryCategories,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageRegistryCategories {
    pub games: GameRootsRegistry,
    pub artwork: FolderRootsRegistry,
    pub ai_models: FolderRootsRegistry,
    pub updates: FolderRootsRegistry,
    pub logs: FolderRootsRegistry,
    pub temporary: FolderRootsRegistry,
    pub system: FolderRootsRegistry,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameRootsRegistry {
    pub label: String,
    pub roots: Vec<GameRoot>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameRoot {
    pub id: String,
    pub platform: String,
    pub display_name: String,
    pub path: String,
    pub samba_share_name: String,
    #[serde(rename = "windowsUNC")]
    pub windows_unc: Option<String>,
    #[serde(rename = "windowsUNCByIp")]
    pub windows_unc_by_ip: Option<String>,
    pub smb_url: Option<String>,
    pub smb_url_by_ip: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderRootsRegistry {
    pub label: String,
    pub roots: Vec<FolderRoot>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderRoot {
    pub id: String,
    pub display_name: String,
    pub path: String,
    pub purpose: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageCategories {
    pub games: StorageCategoryStatus,
    pub artwork: StorageCategoryStatus,
    pub ai_models: StorageCategoryStatus,
    pub updates: StorageCategoryStatus,
    pub logs: StorageCategoryStatus,
    pub temporary: StorageCategoryStatus,
    pub system: StorageCategoryStatus,
    pub other: StorageCategoryStatus,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageCategoryStatus {
    pub bytes: u64,
    pub size: String,
    pub files: u64,
    pub file_count: Option<u64>,
    pub root_count: usize,
    pub state: String,
    pub meta: String,
    pub detail: String,
    pub percent_of_used: u8,
    pub percent_of_total: u8,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameFolderStorage {
    pub platform: String,
    pub display_name: String,
    pub path: String,
    pub samba_share_name: String,
    pub bytes: u64,
    pub size: String,
    pub file_count: u64,
    pub synced_entries: Option<u64>,
    pub unsynced_files: Option<u64>,
    pub largest_files: Vec<LargestFile>,
    #[serde(rename = "windowsUNC")]
    pub windows_unc: Option<String>,
    #[serde(rename = "windowsUNCByIp")]
    pub windows_unc_by_ip: Option<String>,
    pub smb_url: Option<String>,
    pub smb_url_by_ip: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LargestFile {
    pub name: String,
    pub path: String,
    pub bytes: u64,
    pub size: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderStorage {
    pub id: String,
    pub display_name: String,
    pub path: String,
    pub bytes: u64,
    pub size: String,
    pub file_count: u64,
    pub last_modified_at: Option<String>,
    pub state: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AIModelStorage {
    pub id: String,
    pub name: String,
    pub filename: String,
    pub path: String,
    pub bytes: u64,
    pub size: String,
    pub source: String,
    pub loaded: bool,
    pub selected: bool,
    pub removable: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupState {
    pub artwork_bytes_clearable: u64,
    pub temporary_bytes_clearable: u64,
    pub partial_downloads_bytes_clearable: u64,
    pub old_update_bytes_clearable: u64,
    pub logs_bytes_clearable: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageDiagnostics {
    pub mount_point: String,
    pub filesystem: Option<String>,
    pub scan_duration_ms: u64,
    pub scanner_version: String,
    pub missing_dirs: Vec<String>,
    pub permission_errors: Vec<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiModelStorageStatus {
    pub bytes: u64,
    pub size: String,
    pub count: usize,
    pub meta: String,
    pub detail: String,
    pub percent_of_total: u8,
    pub models: Vec<AiModelDiskStatus>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiModelDiskStatus {
    pub friendly_name: String,
    pub filename: String,
    pub size: String,
    pub status: &'static str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RescanFolderRequest {
    path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateManagedFolderRequest {
    path: String,
}

#[derive(Clone, Serialize)]
pub struct SurfaceStatus {
    pub http: String,
    pub mdns: String,
    pub smb: String,
    #[serde(rename = "windowsUNC")]
    pub windows_unc: Option<String>,
    #[serde(rename = "windowsUNCByIp")]
    pub windows_unc_by_ip: Option<String>,
    pub smb_url: Option<String>,
    pub smb_url_by_ip: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct SambaStatus {
    pub state: String,
    pub shares: Vec<SambaShareStatus>,
}

#[derive(Clone, Serialize)]
pub struct SambaShareStatus {
    pub name: String,
    pub purpose: String,
    #[serde(rename = "windowsUNC")]
    pub windows_unc: Option<String>,
    #[serde(rename = "windowsUNCByIp")]
    pub windows_unc_by_ip: Option<String>,
    pub smb_url: Option<String>,
    pub smb_url_by_ip: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct UpdatesStatus {
    pub state: String,
    pub current_version: String,
    pub available_version: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct NetworkStatus {
    pub online: bool,
    pub active_type: String,
    pub connection_type: String,
    pub ssid: Option<String>,
    pub ip_address: String,
    pub gateway: Option<String>,
    pub dns_status: String,
    pub signal: Option<String>,
    pub signal_percent: Option<u8>,
    pub ethernet_speed_mbps: Option<u64>,
    pub ethernet_available: bool,
    pub ethernet_connected: bool,
    pub ethernet_mac_address: Option<String>,
    pub ethernet_dhcp: bool,
    pub wifi_adapter_available: bool,
    pub console_reachable: bool,
    pub game_folders_reachable: bool,
    pub samba_reachable: bool,
    pub lan_ai_reachable: bool,
    pub internet_reachable: Option<bool>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkState {
    pub appliance: NetworkAppliance,
    pub active_connection: ActiveConnection,
    pub ethernet: EthernetState,
    pub wifi: WifiState,
    pub services: NetworkServices,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkAppliance {
    pub product_name: String,
    pub hostname: String,
    pub local_domain: Option<String>,
    pub netbios_name: Option<String>,
    pub web_origin: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveConnection {
    #[serde(rename = "type")]
    pub connection_type: String,
    pub interface_name: Option<String>,
    pub ip: Option<String>,
    pub prefix_length: Option<u8>,
    pub gateway: Option<String>,
    pub dns_servers: Vec<String>,
    pub internet_reachable: Option<bool>,
    pub lan_reachable: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EthernetState {
    pub available: bool,
    pub connected: bool,
    pub interface_name: Option<String>,
    pub mac_address: Option<String>,
    pub speed_mbps: Option<u64>,
    pub ip: Option<String>,
    pub dhcp: bool,
    pub gateway: Option<String>,
    pub dns_servers: Vec<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WifiState {
    pub adapter_available: bool,
    pub enabled: bool,
    pub scanning: bool,
    pub connected_ssid: Option<String>,
    pub signal_percent: Option<u8>,
    pub security: Option<String>,
    pub saved_networks: Vec<SavedWifiNetwork>,
    pub scan_results: Vec<WifiScanResult>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedWifiNetwork {
    pub ssid: String,
    pub security: Option<String>,
    pub last_connected_at: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WifiScanResult {
    pub ssid: String,
    pub bssid: Option<String>,
    pub signal_percent: u8,
    pub security: String,
    pub saved: bool,
    pub connected: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkServices {
    pub web_console: WebConsoleService,
    pub samba: SambaServiceState,
    pub lan_inference: LanInferenceService,
    pub ssh: SshService,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebConsoleService {
    pub state: String,
    pub urls: Vec<String>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SambaServiceState {
    pub state: String,
    pub shares: Vec<SambaShareStatus>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LanInferenceService {
    pub state: String,
    pub port: u16,
    pub urls: Vec<String>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SshService {
    pub state: String,
    pub port: u16,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NetworkActionResponse {
    ok: bool,
    action: &'static str,
    message: String,
    stage: Option<String>,
    state: NetworkState,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticsResponse {
    ok: bool,
    action: &'static str,
    results: Vec<DiagnosticResult>,
    state: NetworkState,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticResult {
    name: String,
    ok: bool,
    message: String,
    detail: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WifiSetEnabledRequest {
    enabled: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IpApplyRequest {
    mode: String,
    interface_name: Option<String>,
    ip: Option<String>,
    prefix_length: Option<u8>,
    gateway: Option<String>,
    dns_servers: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IpConfirmRequest {
    token: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticsRequest {
    tests: Option<Vec<String>>,
}

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
    pub is_inharmonia: bool,
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
    pub is_inharmonia: bool,
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
    pub host: String,
    pub port: u16,
    pub endpoint_urls: Vec<String>,
    pub api_mode: Option<String>,
    pub request_count: Option<u64>,
    pub last_request_at: Option<String>,
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
struct AIRecommendedInstallRequest {
    id: String,
}

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

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Danger,
}
impl ButtonVariant {
    pub fn class(self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::Secondary => "secondary",
            Self::Danger => "danger",
        }
    }
}

#[derive(Deserialize)]
struct GuiPinUnlockRequest {
    pin: String,
}

#[derive(Deserialize)]
struct GuiPinAccessRequest {
    pin_required: bool,
}

#[derive(Deserialize)]
struct PinChangeRequest {
    current_pin: String,
    new_pin: String,
}

#[derive(Deserialize)]
struct PinResetRequest {
    confirm: String,
}

#[derive(Deserialize)]
struct ProviderKeysRequest {
    steamgriddb_api_key: Option<String>,
    thegamesdb_api_key: Option<String>,
    screenscraper_api_key: Option<String>,
}

#[derive(Serialize)]
struct ProviderKeysResponse {
    ok: bool,
    action: &'static str,
    path: &'static str,
    written_keys: Vec<&'static str>,
    message: String,
}

#[derive(Deserialize)]
struct ConsoleActionRequest {
    confirm: Option<String>,
}

#[derive(Serialize)]
struct ConsoleActionResponse {
    ok: bool,
    action: &'static str,
    command: &'static str,
    exit_code: Option<i32>,
    message: String,
    stdout: String,
    stderr: String,
}

#[derive(Serialize)]
struct GuiPinActionResponse {
    ok: bool,
    pin_required: bool,
    action: &'static str,
    helper_present: bool,
    message: String,
}

#[derive(Deserialize)]
struct WifiConnectRequest {
    ssid: String,
    password: Option<String>,
}
#[derive(Deserialize)]
struct WifiForgetRequest {
    ssid: String,
}

#[tokio::main]
async fn main() -> anyhow_free::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let bind = env::var("ARCADIA_BIND").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    let addr: SocketAddr = bind.parse()?;
    let started_unix = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let state = Arc::new(AppState {
        started_unix,
        canonical_url: env::var("ARCADIA_CANONICAL_URL")
            .unwrap_or_else(|_| "http://console.home.arpa/".to_string()),
        product: "HomeConsole".to_string(),
    });

    let app = Router::new()
        .route("/", get(index))
        .route("/health", get(health))
        .route("/api/status", get(status))
        .route("/api/storage/state", get(storage_state_route))
        .route("/api/storage/registry", get(storage_registry_route))
        .route("/api/storage/rescan", post(storage_rescan_route))
        .route(
            "/api/storage/rescan-folder",
            post(storage_rescan_folder_route),
        )
        .route(
            "/api/storage/cleanup/artwork",
            post(storage_cleanup_artwork_route),
        )
        .route(
            "/api/storage/cleanup/temporary",
            post(storage_cleanup_temporary_route),
        )
        .route(
            "/api/storage/cleanup/partial-ai-downloads",
            post(storage_cleanup_partial_ai_downloads_route),
        )
        .route(
            "/api/storage/cleanup/old-updates",
            post(storage_cleanup_old_updates_route),
        )
        .route(
            "/api/storage/cleanup/logs",
            post(storage_cleanup_logs_route),
        )
        .route(
            "/api/storage/create-managed-folder",
            post(storage_create_managed_folder_route),
        )
        .route("/api/storage/game-folders", get(storage_game_folders_route))
        .route("/api/storage/artwork", get(storage_artwork_route))
        .route("/api/storage/ai-models", get(storage_ai_models_route))
        .route("/api/network/state", get(network_state_route))
        .route("/api/ai/state", get(ai_state_route))
        .route("/api/network/wifi/status", get(wifi_status))
        .route("/api/network/wifi/scan", post(wifi_scan))
        .route("/api/network/wifi/connect", post(wifi_connect))
        .route("/api/network/wifi/disconnect", post(wifi_disconnect))
        .route("/api/network/wifi/forget", post(wifi_forget))
        .route("/api/network/wifi/set-enabled", post(wifi_set_enabled))
        .route(
            "/api/network/ethernet/renew-dhcp",
            post(ethernet_renew_dhcp),
        )
        .route("/api/network/ip/apply", post(ip_apply))
        .route("/api/network/ip/confirm", post(ip_confirm))
        .route("/api/network/ip/rollback", post(ip_rollback))
        .route("/api/network/diagnostics/run", post(diagnostics_run))
        .route("/api/gui-pin/status", get(gui_pin_status_route))
        .route("/api/gui-pin/access", post(set_gui_pin_access))
        .route("/api/gui-pin/change", post(change_gui_pin))
        .route("/api/provider-keys/save", post(save_provider_keys))
        .route(
            "/api/ai/runtime/check-update",
            post(ai_runtime_check_update),
        )
        .route("/api/ai/runtime/update", post(ai_runtime_update))
        .route("/api/ai/runtime/restart", post(ai_runtime_restart))
        .route("/api/ai/models/installed", get(ai_models_installed))
        .route("/api/ai/models/recommended", get(ai_models_recommended))
        .route(
            "/api/ai/models/install-recommended",
            post(ai_install_recommended),
        )
        .route(
            "/api/ai/models/huggingface/list-files",
            post(ai_hf_list_files),
        )
        .route("/api/ai/models/huggingface/download", post(ai_hf_download))
        .route("/api/ai/models/download/cancel", post(ai_download_cancel))
        .route("/api/ai/models/remove", post(ai_model_remove))
        .route("/api/ai/model/select", post(ai_model_select))
        .route("/api/ai/model/load", post(ai_model_load))
        .route("/api/ai/model/unload", post(ai_model_unload))
        .route(
            "/api/ai/inference/set-enabled",
            post(ai_inference_set_enabled),
        )
        .route(
            "/api/ai/inference/set-lan-access",
            post(ai_inference_set_lan_access),
        )
        .route("/api/ai/inference/test", post(ai_inference_test))
        .route("/api/actions/update-gui", post(action_update_gui))
        .route("/api/actions/sync-games", post(action_sync_games))
        .route(
            "/api/actions/clear-artwork-cache",
            post(action_clear_artwork_cache),
        )
        .route(
            "/api/actions/clean-temporary-files",
            post(action_clean_temporary_files),
        )
        .route("/api/actions/remove-ai-model", post(action_remove_ai_model))
        .route("/api/actions/reboot-console", post(action_reboot_console))
        .route(
            "/api/actions/shutdown-console",
            post(action_shutdown_console),
        )
        .route(
            "/api/actions/restart-gamescope",
            post(action_restart_gamescope),
        )
        .route("/api/gui-pin/reset-default", post(reset_gui_pin_default))
        .route("/pre-unlock", post(pre_unlock))
        .route("/static/app.css", get(css))
        .route("/static/app.js", get(js))
        .fallback(not_found)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = TcpListener::bind(addr).await?;
    tracing::info!(%addr, "arcadia listening");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn index(State(state): State<Arc<AppState>>) -> maud::Markup {
    ui::layout(&console_status(&state))
}

async fn health(State(state): State<Arc<AppState>>) -> Json<Health> {
    Json(Health {
        ok: true,
        service: "arcadia",
        product: state.product.clone(),
        version: env!("CARGO_PKG_VERSION"),
        started_unix: state.started_unix,
    })
}

async fn status(State(state): State<Arc<AppState>>) -> Json<ConsoleStatus> {
    Json(console_status(&state))
}

async fn storage_state_route() -> Json<StorageStatus> {
    Json(storage_status())
}

async fn storage_registry_route() -> Json<StorageRegistry> {
    Json(storage_registry(&network_status()))
}

async fn storage_rescan_route() -> Json<StorageStatus> {
    Json(storage_status())
}

async fn storage_rescan_folder_route(
    Json(body): Json<RescanFolderRequest>,
) -> (StatusCode, Json<FolderStorage>) {
    if !is_managed_storage_path(&body.path) {
        return (
            StatusCode::BAD_REQUEST,
            Json(folder_storage("unmanaged", "Unmanaged", &body.path)),
        );
    }
    (
        StatusCode::OK,
        Json(folder_storage("managed", "Managed folder", &body.path)),
    )
}

async fn storage_game_folders_route() -> Json<Vec<GameFolderStorage>> {
    Json(storage_scan().game_folders)
}

async fn storage_artwork_route() -> Json<Vec<FolderStorage>> {
    Json(storage_scan().artwork_stores)
}

async fn storage_ai_models_route() -> Json<Vec<AIModelStorage>> {
    Json(storage_scan().ai_model_files)
}

async fn storage_cleanup_artwork_route(
    Json(body): Json<ConsoleActionRequest>,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    action_clear_artwork_cache(Json(body)).await
}

async fn storage_cleanup_temporary_route(
    Json(body): Json<ConsoleActionRequest>,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    action_clean_temporary_files(Json(body)).await
}

async fn storage_cleanup_partial_ai_downloads_route(
    Json(body): Json<ConsoleActionRequest>,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    if body.confirm.as_deref() != Some("CLEAR_PARTIAL_DOWNLOADS") {
        return console_action_error(
            StatusCode::BAD_REQUEST,
            "clear-partial-ai-downloads",
            "arcadia-storage",
            "Confirm before clearing partial AI downloads.",
        );
    }
    cleanup_known_roots(
        "clear-partial-ai-downloads",
        partial_ai_download_roots(),
        "Partial AI downloads cleared. Installed models were not removed.",
    )
}

async fn storage_cleanup_old_updates_route(
    Json(body): Json<ConsoleActionRequest>,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    if body.confirm.as_deref() != Some("CLEAR_OLD_UPDATES") {
        return console_action_error(
            StatusCode::BAD_REQUEST,
            "clear-old-updates",
            "arcadia-storage",
            "Confirm before clearing old update packages.",
        );
    }
    cleanup_known_roots(
        "clear-old-updates",
        update_roots(),
        "Old update packages cleared.",
    )
}

async fn storage_cleanup_logs_route(
    Json(body): Json<ConsoleActionRequest>,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    if body.confirm.as_deref() != Some("PRUNE_LOGS") {
        return console_action_error(
            StatusCode::BAD_REQUEST,
            "prune-logs",
            "arcadia-storage",
            "Confirm before pruning managed logs.",
        );
    }
    cleanup_known_roots("prune-logs", log_roots(), "Managed logs pruned.")
}

async fn storage_create_managed_folder_route(
    Json(body): Json<CreateManagedFolderRequest>,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    if !is_managed_storage_path(&body.path) {
        return console_action_error(
            StatusCode::BAD_REQUEST,
            "create-managed-folder",
            "arcadia-storage",
            "Only configured managed storage roots can be created.",
        );
    }
    match fs::create_dir_all(&body.path) {
        Ok(()) => (
            StatusCode::OK,
            Json(ConsoleActionResponse {
                ok: true,
                action: "create-managed-folder",
                command: "arcadia-storage",
                exit_code: Some(0),
                message: format!("Managed folder created: {}", body.path),
                stdout: String::new(),
                stderr: String::new(),
            }),
        ),
        Err(err) => console_action_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "create-managed-folder",
            "arcadia-storage",
            &format!("Managed folder could not be created: {err}"),
        ),
    }
}

async fn network_state_route(State(state): State<Arc<AppState>>) -> Json<NetworkState> {
    Json(network_state(&state))
}

async fn ai_state_route(State(state): State<Arc<AppState>>) -> Json<LocalAIState> {
    Json(local_ai_state(&state))
}

async fn ai_models_installed(State(state): State<Arc<AppState>>) -> Json<Vec<LocalAiModelStatus>> {
    Json(local_ai_state(&state).installed_models)
}
async fn ai_models_recommended(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<AIRecommendedModel>> {
    Json(local_ai_state(&state).recommended_models)
}

async fn gui_pin_status_route() -> Json<GuiPinStatus> {
    Json(gui_pin_status())
}

async fn ai_runtime_check_update(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<AIActionResponse>) {
    ai_action(
        StatusCode::OK,
        &state,
        true,
        "runtime-check-update",
        "Runtime update check completed.",
    )
}
async fn ai_runtime_update(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<AIActionResponse>) {
    if helper_exists("/usr/local/bin/harmonia") {
        ai_action(
            StatusCode::OK,
            &state,
            true,
            "runtime-update",
            "Runtime update started through Harmonia.",
        )
    } else {
        ai_action(
            StatusCode::NOT_IMPLEMENTED,
            &state,
            false,
            "runtime-update",
            "Runtime update helper is unavailable.",
        )
    }
}
async fn ai_runtime_restart(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<AIActionResponse>) {
    let ok = Command::new(SYSTEMCTL_BIN)
        .args(["--user", "restart", "llama-server.service"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
        || Command::new(SYSTEMCTL_BIN)
            .args(["restart", "llama-server.service"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
    ai_action(
        if ok {
            StatusCode::OK
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        },
        &state,
        ok,
        "runtime-restart",
        if ok {
            "Runtime restarted."
        } else {
            "Runtime restart failed or service is not installed."
        },
    )
}
async fn ai_install_recommended(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AIRecommendedInstallRequest>,
) -> (StatusCode, Json<AIActionResponse>) {
    if body.id != "inharmonia" {
        return ai_action(
            StatusCode::BAD_REQUEST,
            &state,
            false,
            "install-recommended",
            "Unknown recommended model.",
        );
    }
    if local_ai_available_models().iter().any(|m| m.is_inharmonia) {
        return ai_action(
            StatusCode::OK,
            &state,
            true,
            "install-recommended",
            "Inharmonia is already installed.",
        );
    }
    ai_action(
        StatusCode::FAILED_DEPENDENCY,
        &state,
        false,
        "install-recommended",
        "Inharmonia catalog source is not configured on this console yet.",
    )
}
async fn ai_hf_list_files(
    State(state): State<Arc<AppState>>,
    Json(body): Json<HFListFilesRequest>,
) -> (StatusCode, Json<HFFileListResponse>) {
    let repo = body.repo_id.trim();
    if !valid_hf_repo(repo) {
        return (
            StatusCode::BAD_REQUEST,
            Json(HFFileListResponse {
                ok: false,
                message: "Enter a Hugging Face repository like owner/model.".into(),
                files: Vec::new(),
                state: local_ai_state(&state),
            }),
        );
    }
    let rev = body.revision.as_deref().unwrap_or("main");
    let url = format!(
        "https://huggingface.co/api/models/{}/tree/{}?recursive=1",
        repo, rev
    );
    let text = command_stdout("curl", &["-fsSL", "--max-time", "20", &url]).unwrap_or_default();
    let mut files = Vec::new();
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
        if let Some(arr) = value.as_array() {
            for item in arr {
                let path = item
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                if !path.to_ascii_lowercase().ends_with(".gguf") {
                    continue;
                }
                let size = item.get("size").and_then(|v| v.as_u64());
                files.push(HFModelFile {
                    filename: path.to_string(),
                    size_bytes: size,
                    estimated_vram_bytes: size.map(|v| v.saturating_add(v / 5)),
                });
            }
        }
    }
    let ok = !files.is_empty();
    (
        StatusCode::OK,
        Json(HFFileListResponse {
            ok,
            message: if ok {
                "Compatible .gguf files found."
            } else {
                "No compatible .gguf files found or repository requires access."
            }
            .into(),
            files,
            state: local_ai_state(&state),
        }),
    )
}
async fn ai_hf_download(
    State(state): State<Arc<AppState>>,
    Json(body): Json<HFDownloadRequest>,
) -> (StatusCode, Json<AIActionResponse>) {
    if !valid_hf_repo(&body.repo_id)
        || !body.filename.to_ascii_lowercase().ends_with(".gguf")
        || body.filename.contains("..")
    {
        return ai_action(
            StatusCode::BAD_REQUEST,
            &state,
            false,
            "huggingface-download",
            "Only compatible .gguf model files can be downloaded.",
        );
    }
    let rev = body.revision.as_deref().unwrap_or("main");
    let name = Path::new(&body.filename)
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or("model.gguf");
    let _ = fs::create_dir_all(LOCAL_AI_MODEL_ROOT);
    let out = Path::new(LOCAL_AI_MODEL_ROOT).join(name);
    let url = format!(
        "https://huggingface.co/{}/resolve/{}/{}",
        body.repo_id, rev, body.filename
    );
    let ok = Command::new("curl")
        .args([
            "-fL",
            "--progress-bar",
            "-o",
            out.to_string_lossy().as_ref(),
            &url,
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    ai_action(
        if ok {
            StatusCode::OK
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        },
        &state,
        ok,
        "huggingface-download",
        if ok {
            "Model downloaded and installed."
        } else {
            "Model download failed. Check repository access, license, token requirements, or disk space."
        },
    )
}
async fn ai_download_cancel(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<AIActionResponse>) {
    ai_action(
        StatusCode::OK,
        &state,
        true,
        "download-cancel",
        "No active download is running.",
    )
}
async fn ai_model_remove(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AIModelIdRequest>,
) -> (StatusCode, Json<AIActionResponse>) {
    if body.confirm.as_deref() != Some("REMOVE_MODEL") {
        return ai_action(
            StatusCode::BAD_REQUEST,
            &state,
            false,
            "model-remove",
            "Remove this model from console storage? Games and artwork are not affected.",
        );
    }
    let filename = body
        .filename
        .or_else(|| {
            body.model_id.and_then(|id| {
                local_ai_available_models()
                    .into_iter()
                    .find(|m| m.id == id)
                    .map(|m| m.filename)
            })
        })
        .unwrap_or_default();
    let loaded = local_ai_status().loaded_model.unwrap_or_default();
    if loaded == filename {
        return ai_action(
            StatusCode::BAD_REQUEST,
            &state,
            false,
            "model-remove",
            "Unload before removing this model.",
        );
    }
    match find_model_path_by_filename(&filename).and_then(|p| fs::remove_file(p).ok().map(|_| ())) {
        Some(_) => ai_action(
            StatusCode::OK,
            &state,
            true,
            "model-remove",
            "Model removed from console storage.",
        ),
        None => ai_action(
            StatusCode::NOT_FOUND,
            &state,
            false,
            "model-remove",
            "Model file was not found.",
        ),
    }
}
async fn ai_model_select(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AIModelIdRequest>,
) -> (StatusCode, Json<AIActionResponse>) {
    let id = body.model_id.unwrap_or_default();
    if local_ai_available_models().iter().any(|m| m.id == id) {
        let _ = fs::create_dir_all(
            Path::new(LOCAL_AI_STATE_PATH)
                .parent()
                .unwrap_or(Path::new("/var/lib/arcadia")),
        );
        let _ = fs::write(
            LOCAL_AI_STATE_PATH,
            format!("{{\"selectedModelId\":\"{}\"}}", id),
        );
        ai_action(
            StatusCode::OK,
            &state,
            true,
            "model-select",
            "Model selected.",
        )
    } else {
        ai_action(
            StatusCode::NOT_FOUND,
            &state,
            false,
            "model-select",
            "Selected model is not installed.",
        )
    }
}
async fn ai_model_load(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AIModelIdRequest>,
) -> (StatusCode, Json<AIActionResponse>) {
    let id = body.model_id.unwrap_or_else(|| {
        local_ai_state(&state)
            .loaded_model
            .selected_model_id
            .unwrap_or_default()
    });
    let Some(model) = local_ai_available_models().into_iter().find(|m| m.id == id) else {
        return ai_action(
            StatusCode::NOT_FOUND,
            &state,
            false,
            "model-load",
            "No installed model is selected.",
        );
    };
    let Some(path) = find_model_path_by_filename(&model.filename) else {
        return ai_action(
            StatusCode::NOT_FOUND,
            &state,
            false,
            "model-load",
            "Model file was not found.",
        );
    };
    let ok = helper_exists(LLAMA_SERVER_BIN)
        && Command::new(SYSTEMD_RUN_BIN)
            .args([
                "--unit=arcadia-llama-server",
                "--collect",
                LLAMA_SERVER_BIN,
                "-m",
                path.to_string_lossy().as_ref(),
                "--port",
                &LAN_INFERENCE_PORT.to_string(),
                "--host",
                "127.0.0.1",
            ])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
    ai_action(
        if ok {
            StatusCode::OK
        } else {
            StatusCode::FAILED_DEPENDENCY
        },
        &state,
        ok,
        "model-load",
        if ok {
            "Model load started."
        } else {
            "llama.cpp server is missing or the model could not be started."
        },
    )
}
async fn ai_model_unload(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<AIActionResponse>) {
    let ok = Command::new("pkill")
        .args(["-f", "llama-server"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    ai_action(
        StatusCode::OK,
        &state,
        true,
        "model-unload",
        if ok {
            "Model unloaded."
        } else {
            "No loaded model was running."
        },
    )
}
async fn ai_inference_set_enabled(
    State(state): State<Arc<AppState>>,
    Json(body): Json<InferenceSetRequest>,
) -> (StatusCode, Json<AIActionResponse>) {
    ai_action(
        StatusCode::OK,
        &state,
        true,
        "inference-set-enabled",
        if body.enabled {
            "Inference enabled for Local AI."
        } else {
            "Inference disabled for Local AI."
        },
    )
}
async fn ai_inference_set_lan_access(
    State(state): State<Arc<AppState>>,
    Json(body): Json<InferenceSetRequest>,
) -> (StatusCode, Json<AIActionResponse>) {
    ai_action(
        StatusCode::OK,
        &state,
        true,
        "inference-set-lan-access",
        if body.enabled {
            "LAN access enabled for trusted home networks only. Do not expose port 7777 to the public internet."
        } else {
            "LAN access disabled."
        },
    )
}
async fn ai_inference_test(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<AIActionResponse>) {
    let ok = tcp_port_listening(LAN_INFERENCE_PORT);
    ai_action(
        if ok {
            StatusCode::OK
        } else {
            StatusCode::FAILED_DEPENDENCY
        },
        &state,
        ok,
        "inference-test",
        if ok {
            "Inference endpoint is listening."
        } else {
            "No model is serving inference on port 7777."
        },
    )
}
fn ai_action(
    status: StatusCode,
    state: &AppState,
    ok: bool,
    action: &'static str,
    message: &str,
) -> (StatusCode, Json<AIActionResponse>) {
    (
        status,
        Json(AIActionResponse {
            ok,
            action,
            message: message.to_string(),
            state: local_ai_state(state),
        }),
    )
}
fn valid_hf_repo(repo: &str) -> bool {
    repo.split('/').count() == 2
        && !repo.contains("..")
        && repo
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '-' | '_' | '.'))
}

async fn action_update_gui() -> (StatusCode, Json<ConsoleActionResponse>) {
    run_console_command(
        "update-gui",
        SYSTEMD_RUN_BIN,
        &[
            "--unit=arcadia-gui-update",
            "--collect",
            HARMONIA_BIN,
            "homeconsole-arcadia-gui-update",
            HOMECONSOLE_PROFILE,
            "--repo",
            "git@git.home.arpa:HOMESERVERSLTD/arcadia.git",
            "--branch",
            "main",
            "--source-dir",
            "/opt/arcadia/source",
            "--apply",
            "--install-bin",
            "/usr/local/bin/arcadia",
            "--service",
            "arcadia.service",
            "--receipt-dir",
            "/var/lib/harmonia/receipts/arcadia-gui-latest",
        ],
        "Update GUI started. Read /var/lib/harmonia/receipts/arcadia-gui-latest after Arcadia restarts.",
        "Update GUI could not start.",
    )
}

async fn action_sync_games() -> (StatusCode, Json<ConsoleActionResponse>) {
    run_console_command(
        "sync-games",
        HARMONIA_BIN,
        &[
            "homeconsole-sync",
            HOMECONSOLE_PROFILE,
            "--provider-env",
            PROVIDER_KEYS_PATH,
            "--adapter-command",
            ARCH_GAME_SYNC_BIN,
            "--apply",
            "--receipt-dir",
            "/var/lib/harmonia/receipts/game-sync-latest",
        ],
        "Sync games completed.",
        "Sync games failed. Read /var/lib/harmonia/receipts/game-sync-latest.",
    )
}

async fn action_clear_artwork_cache(
    Json(body): Json<ConsoleActionRequest>,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    if body.confirm.as_deref() != Some("CLEAR_ARTWORK") {
        return console_action_error(
            StatusCode::BAD_REQUEST,
            "clear-artwork-cache",
            "arcadia-storage",
            "Confirm before clearing artwork cache. This does not delete games.",
        );
    }
    storage_remove_children(
        "clear-artwork-cache",
        ARTWORK_ROOT,
        "Artwork cache cleared. Games were not deleted.",
        "Artwork cache could not be cleared.",
    )
}

async fn action_clean_temporary_files(
    Json(body): Json<ConsoleActionRequest>,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    if body.confirm.as_deref() != Some("CLEAN_TEMPORARY") {
        return console_action_error(
            StatusCode::BAD_REQUEST,
            "clean-temporary-files",
            "arcadia-storage",
            "Confirm before cleaning safe temporary files.",
        );
    }
    let mut removed = 0u64;
    let mut errors = Vec::new();
    for root in TEMP_CLEAN_ROOTS {
        match remove_children(Path::new(root)) {
            Ok(count) => removed += count,
            Err(err) => errors.push(format!("{}: {}", root, err)),
        }
    }
    let ok = errors.is_empty();
    let message = if ok {
        format!("Cleaned safe temporary files. Removed {} entries. Games, artwork, and AI models were not touched.", removed)
    } else {
        format!(
            "Temporary cleanup partially failed after removing {} entries: {}",
            removed,
            errors.join("; ")
        )
    };
    (
        if ok {
            StatusCode::OK
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        },
        Json(ConsoleActionResponse {
            ok,
            action: "clean-temporary-files",
            command: "arcadia-storage",
            exit_code: if ok { Some(0) } else { Some(1) },
            message,
            stdout: String::new(),
            stderr: errors.join("\n"),
        }),
    )
}

async fn action_remove_ai_model(
    Query(params): Query<HashMap<String, String>>,
    Json(body): Json<ConsoleActionRequest>,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    if body.confirm.as_deref() != Some("REMOVE_MODEL") {
        return console_action_error(
            StatusCode::BAD_REQUEST,
            "remove-ai-model",
            "arcadia-storage",
            "Confirm before removing a local AI model file. This does not affect games.",
        );
    }
    let Some(filename) = params.get("name") else {
        return console_action_error(
            StatusCode::BAD_REQUEST,
            "remove-ai-model",
            "arcadia-storage",
            "Missing model filename.",
        );
    };
    let current = storage_status();
    let Some(path) = find_model_path_by_filename(filename) else {
        return console_action_error(
            StatusCode::NOT_FOUND,
            "remove-ai-model",
            "arcadia-storage",
            "Model file was not found in local AI storage.",
        );
    };
    let model_known = current
        .ai_models
        .models
        .iter()
        .any(|model| model.filename == *filename);
    if !model_known {
        return console_action_error(
            StatusCode::NOT_FOUND,
            "remove-ai-model",
            "arcadia-storage",
            "Model file was not found in the current storage inventory.",
        );
    }
    match fs::remove_file(&path) {
        Ok(()) => (
            StatusCode::OK,
            Json(ConsoleActionResponse {
                ok: true,
                action: "remove-ai-model",
                command: "arcadia-storage",
                exit_code: Some(0),
                message: format!(
                    "Removed {} from console storage. Games were not affected.",
                    filename
                ),
                stdout: String::new(),
                stderr: String::new(),
            }),
        ),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ConsoleActionResponse {
                ok: false,
                action: "remove-ai-model",
                command: "arcadia-storage",
                exit_code: Some(1),
                message: format!("Could not remove {}.", filename),
                stdout: String::new(),
                stderr: err.to_string(),
            }),
        ),
    }
}

async fn action_reboot_console(
    Json(body): Json<ConsoleActionRequest>,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    if body.confirm.as_deref() != Some("REBOOT") {
        return console_action_error(
            StatusCode::BAD_REQUEST,
            "reboot-console",
            SYSTEMCTL_BIN,
            "Type REBOOT to restart the console.",
        );
    }
    run_console_command(
        "reboot-console",
        SYSTEMCTL_BIN,
        &["reboot"],
        "Reboot requested.",
        "Reboot request failed.",
    )
}

async fn action_shutdown_console(
    Json(body): Json<ConsoleActionRequest>,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    if body.confirm.as_deref() != Some("SHUTDOWN") {
        return console_action_error(
            StatusCode::BAD_REQUEST,
            "shutdown-console",
            SYSTEMCTL_BIN,
            "Type SHUTDOWN to power off the console.",
        );
    }
    run_console_command(
        "shutdown-console",
        SYSTEMCTL_BIN,
        &["poweroff"],
        "Shutdown requested.",
        "Shutdown request failed.",
    )
}

async fn action_restart_gamescope(
    Json(body): Json<ConsoleActionRequest>,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    if body.confirm.as_deref() != Some("RESTART_GAMESCOPE") {
        return console_action_error(
            StatusCode::BAD_REQUEST,
            "restart-gamescope",
            SYSTEMCTL_BIN,
            "Confirm GameScope restart before closing the active game session.",
        );
    }
    run_console_command(
        "restart-gamescope",
        SYSTEMCTL_BIN,
        &["restart", "gamescope.service"],
        "GameScope restart requested.",
        "GameScope restart request failed.",
    )
}

fn run_console_command(
    action: &'static str,
    command: &'static str,
    args: &[&str],
    success_message: &str,
    failure_message: &str,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    if !helper_exists(command) {
        return console_action_error(
            StatusCode::NOT_IMPLEMENTED,
            action,
            command,
            "Required command is not installed on this console.",
        );
    }
    match Command::new(command).args(args).output() {
        Ok(output) => {
            let ok = output.status.success();
            (
                if ok {
                    StatusCode::OK
                } else {
                    StatusCode::INTERNAL_SERVER_ERROR
                },
                Json(ConsoleActionResponse {
                    ok,
                    action,
                    command,
                    exit_code: output.status.code(),
                    message: if ok { success_message } else { failure_message }.to_string(),
                    stdout: redacted_output(&output.stdout),
                    stderr: redacted_output(&output.stderr),
                }),
            )
        }
        Err(err) => console_action_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            action,
            command,
            &format!("Command could not start: {err}"),
        ),
    }
}

fn console_action_error(
    status: StatusCode,
    action: &'static str,
    command: &'static str,
    message: &str,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    (
        status,
        Json(ConsoleActionResponse {
            ok: false,
            action,
            command,
            exit_code: None,
            message: message.to_string(),
            stdout: String::new(),
            stderr: String::new(),
        }),
    )
}

fn redacted_output(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    text.lines()
        .take(80)
        .map(|line| {
            if line.to_ascii_lowercase().contains("key=")
                || line.to_ascii_lowercase().contains("password")
                || line.to_ascii_lowercase().contains("token")
            {
                "[REDACTED]".to_string()
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

async fn save_provider_keys(
    Json(body): Json<ProviderKeysRequest>,
) -> (StatusCode, Json<ProviderKeysResponse>) {
    let mut lines: Vec<String> = Vec::new();
    let mut written: Vec<&'static str> = Vec::new();
    push_env_value(
        &mut lines,
        &mut written,
        "STEAMGRIDDB_API_KEY",
        body.steamgriddb_api_key,
    );
    push_env_value(
        &mut lines,
        &mut written,
        "THEGAMESDB_API_KEY",
        body.thegamesdb_api_key,
    );
    push_env_value(
        &mut lines,
        &mut written,
        "SCREENSCRAPER_API_KEY",
        body.screenscraper_api_key,
    );

    if lines.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ProviderKeysResponse {
                ok: false,
                action: "save-provider-keys",
                path: PROVIDER_KEYS_PATH,
                written_keys: Vec::new(),
                message: "Enter at least one provider key.".to_string(),
            }),
        );
    }

    if let Some(parent) = Path::new(PROVIDER_KEYS_PATH).parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            return provider_keys_error(format!(
                "Provider key directory could not be created: {err}"
            ));
        }
    }

    let write_result = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(PROVIDER_KEYS_PATH)
        .and_then(|mut file| file.write_all(lines.join("\n").as_bytes()))
        .and_then(|_| fs::set_permissions(PROVIDER_KEYS_PATH, provider_file_permissions()));

    match write_result {
        Ok(()) => (
            StatusCode::OK,
            Json(ProviderKeysResponse {
                ok: true,
                action: "save-provider-keys",
                path: PROVIDER_KEYS_PATH,
                written_keys: written,
                message: "Provider keys saved for game sync.".to_string(),
            }),
        ),
        Err(err) => provider_keys_error(format!("Provider keys could not be saved: {err}")),
    }
}

fn push_env_value(
    lines: &mut Vec<String>,
    written: &mut Vec<&'static str>,
    key: &'static str,
    value: Option<String>,
) {
    let Some(value) = value else { return };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return;
    }
    lines.push(format!("{}={}", key, shell_env_quote(trimmed)));
    written.push(key);
}

fn shell_env_quote(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{}\"", escaped)
}

fn provider_keys_error(message: String) -> (StatusCode, Json<ProviderKeysResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ProviderKeysResponse {
            ok: false,
            action: "save-provider-keys",
            path: PROVIDER_KEYS_PATH,
            written_keys: Vec::new(),
            message,
        }),
    )
}

#[cfg(unix)]
fn provider_file_permissions() -> fs::Permissions {
    fs::Permissions::from_mode(0o600)
}

#[cfg(not(unix))]
fn provider_file_permissions() -> fs::Permissions {
    fs::metadata(PROVIDER_KEYS_PATH)
        .map(|m| m.permissions())
        .unwrap_or_else(|_| fs::Permissions::readonly())
}

async fn wifi_status(State(state): State<Arc<AppState>>) -> Json<NetworkState> {
    Json(network_state(&state))
}

async fn wifi_scan(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<NetworkActionResponse>) {
    if !network_state(&state).wifi.adapter_available || !helper_exists(NETWORK_MANAGER_BIN) {
        return network_action(
            StatusCode::NOT_IMPLEMENTED,
            &state,
            false,
            "wifi-scan",
            "Wi-Fi unavailable. No Wi-Fi adapter was detected.",
            Some("adapter-unavailable"),
        );
    }
    match Command::new(NETWORK_MANAGER_BIN)
        .args(["device", "wifi", "rescan"])
        .output()
    {
        Ok(output) if output.status.success() => network_action(
            StatusCode::OK,
            &state,
            true,
            "wifi-scan",
            "Wi-Fi scan complete.",
            Some("scan-complete"),
        ),
        Ok(_) => network_action(
            StatusCode::INTERNAL_SERVER_ERROR,
            &state,
            false,
            "wifi-scan",
            "Wi-Fi scan failed. Try again or use Ethernet.",
            Some("scan"),
        ),
        Err(_) => network_action(
            StatusCode::INTERNAL_SERVER_ERROR,
            &state,
            false,
            "wifi-scan",
            "Wi-Fi manager could not start.",
            Some("scan"),
        ),
    }
}

async fn wifi_connect(
    State(state): State<Arc<AppState>>,
    Json(body): Json<WifiConnectRequest>,
) -> (StatusCode, Json<NetworkActionResponse>) {
    if body.ssid.trim().is_empty() {
        return network_action(
            StatusCode::BAD_REQUEST,
            &state,
            false,
            "wifi-connect",
            "Wi-Fi network name is required.",
            Some("select-network"),
        );
    }
    if !network_state(&state).wifi.adapter_available || !helper_exists(NETWORK_MANAGER_BIN) {
        return network_action(
            StatusCode::NOT_IMPLEMENTED,
            &state,
            false,
            "wifi-connect",
            "Wi-Fi unavailable. No Wi-Fi adapter was detected.",
            Some("adapter-unavailable"),
        );
    }
    let mut args = vec!["device", "wifi", "connect", body.ssid.as_str()];
    if let Some(password) = body.password.as_deref().filter(|value| !value.is_empty()) {
        args.push("password");
        args.push(password);
    }
    let output = Command::new(NETWORK_MANAGER_BIN)
        .args(&args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output();
    match output {
        Ok(output) if output.status.success() => {
            let after = network_state(&state);
            let message = if after.active_connection.ip.is_none() {
                "Connected to Wi-Fi, but no IP address was assigned."
            } else if after.active_connection.internet_reachable == Some(false) {
                "Connected to LAN, but Internet is unavailable."
            } else {
                "Wi-Fi connected."
            };
            (
                StatusCode::OK,
                Json(NetworkActionResponse {
                    ok: true,
                    action: "wifi-connect",
                    message: message.to_string(),
                    stage: Some("testing-internet".to_string()),
                    state: after,
                }),
            )
        }
        Ok(_) => network_action(
            StatusCode::INTERNAL_SERVER_ERROR,
            &state,
            false,
            "wifi-connect",
            "Could not join this Wi-Fi network. Check the password.",
            Some("authenticating"),
        ),
        Err(_) => network_action(
            StatusCode::INTERNAL_SERVER_ERROR,
            &state,
            false,
            "wifi-connect",
            "Wi-Fi manager could not start.",
            Some("joining-network"),
        ),
    }
}

async fn wifi_disconnect(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<NetworkActionResponse>) {
    let Some(dev) = network_state(&state).active_connection.interface_name else {
        return network_action(
            StatusCode::BAD_REQUEST,
            &state,
            false,
            "wifi-disconnect",
            "No active Wi-Fi network is connected.",
            Some("connected-network"),
        );
    };
    match Command::new(NETWORK_MANAGER_BIN)
        .args(["device", "disconnect", dev.as_str()])
        .output()
    {
        Ok(output) if output.status.success() => network_action(
            StatusCode::OK,
            &state,
            true,
            "wifi-disconnect",
            "Wi-Fi disconnected.",
            Some("disconnected"),
        ),
        _ => network_action(
            StatusCode::INTERNAL_SERVER_ERROR,
            &state,
            false,
            "wifi-disconnect",
            "Wi-Fi disconnect failed.",
            Some("disconnect"),
        ),
    }
}

async fn wifi_forget(
    State(state): State<Arc<AppState>>,
    Json(body): Json<WifiForgetRequest>,
) -> (StatusCode, Json<NetworkActionResponse>) {
    if body.ssid.trim().is_empty() {
        return network_action(
            StatusCode::BAD_REQUEST,
            &state,
            false,
            "wifi-forget",
            "Wi-Fi network name is required.",
            Some("saved-network"),
        );
    }
    match Command::new(NETWORK_MANAGER_BIN)
        .args(["connection", "delete", body.ssid.as_str()])
        .output()
    {
        Ok(output) if output.status.success() => network_action(
            StatusCode::OK,
            &state,
            true,
            "wifi-forget",
            "Saved Wi-Fi network removed.",
            Some("forgotten"),
        ),
        _ => network_action(
            StatusCode::INTERNAL_SERVER_ERROR,
            &state,
            false,
            "wifi-forget",
            "Saved Wi-Fi network could not be removed.",
            Some("forget"),
        ),
    }
}

async fn wifi_set_enabled(
    State(state): State<Arc<AppState>>,
    Json(body): Json<WifiSetEnabledRequest>,
) -> (StatusCode, Json<NetworkActionResponse>) {
    let mode = if body.enabled { "on" } else { "off" };
    match Command::new(NETWORK_MANAGER_BIN)
        .args(["radio", "wifi", mode])
        .output()
    {
        Ok(output) if output.status.success() => network_action(
            StatusCode::OK,
            &state,
            true,
            "wifi-set-enabled",
            if body.enabled {
                "Wi-Fi turned on."
            } else {
                "Wi-Fi turned off."
            },
            Some("radio"),
        ),
        _ => network_action(
            StatusCode::INTERNAL_SERVER_ERROR,
            &state,
            false,
            "wifi-set-enabled",
            "Wi-Fi power setting failed.",
            Some("radio"),
        ),
    }
}

async fn ethernet_renew_dhcp(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<NetworkActionResponse>) {
    let ns = network_state(&state);
    let Some(dev) = ns.ethernet.interface_name else {
        return network_action(
            StatusCode::NOT_IMPLEMENTED,
            &state,
            false,
            "ethernet-renew-dhcp",
            "Ethernet unavailable.",
            Some("ethernet"),
        );
    };
    let down = Command::new(NETWORK_MANAGER_BIN)
        .args(["device", "disconnect", dev.as_str()])
        .output();
    let up = Command::new(NETWORK_MANAGER_BIN)
        .args(["device", "connect", dev.as_str()])
        .output();
    if down.is_ok() && up.map(|o| o.status.success()).unwrap_or(false) {
        network_action(
            StatusCode::OK,
            &state,
            true,
            "ethernet-renew-dhcp",
            "DHCP lease renewed.",
            Some("requesting-ip"),
        )
    } else {
        network_action(
            StatusCode::INTERNAL_SERVER_ERROR,
            &state,
            false,
            "ethernet-renew-dhcp",
            "DHCP lease could not be renewed.",
            Some("requesting-ip"),
        )
    }
}

async fn ip_apply(
    State(state): State<Arc<AppState>>,
    Json(body): Json<IpApplyRequest>,
) -> (StatusCode, Json<NetworkActionResponse>) {
    let iface = body
        .interface_name
        .clone()
        .or_else(|| network_state(&state).active_connection.interface_name)
        .unwrap_or_default();
    if iface.is_empty() {
        return network_action(
            StatusCode::BAD_REQUEST,
            &state,
            false,
            "ip-apply",
            "No active interface is available for IP settings.",
            Some("interface"),
        );
    }
    if body.mode == "dhcp" {
        return match Command::new(NETWORK_MANAGER_BIN).args(["connection", "modify", iface.as_str(), "ipv4.method", "auto"]).output() {
            Ok(output) if output.status.success() => network_action(StatusCode::OK, &state, true, "ip-apply", "Network settings changed. Confirm this web GUI remains reachable within 90 seconds.", Some("confirm-reachable")),
            _ => network_action(StatusCode::INTERNAL_SERVER_ERROR, &state, false, "ip-apply", "DHCP settings could not be applied.", Some("apply")),
        };
    }
    if body.mode != "manual" {
        return network_action(
            StatusCode::BAD_REQUEST,
            &state,
            false,
            "ip-apply",
            "Choose Automatic DHCP or Manual IPv4.",
            Some("validate"),
        );
    }
    let Some(ip) = body.ip.as_deref() else {
        return network_action(
            StatusCode::BAD_REQUEST,
            &state,
            false,
            "ip-apply",
            "Manual IPv4 address is required.",
            Some("validate"),
        );
    };
    let Some(prefix) = body.prefix_length else {
        return network_action(
            StatusCode::BAD_REQUEST,
            &state,
            false,
            "ip-apply",
            "Subnet prefix is required.",
            Some("validate"),
        );
    };
    if !valid_ipv4(ip)
        || prefix > 32
        || body.gateway.as_deref().is_some_and(|v| !valid_ipv4(v))
        || body
            .dns_servers
            .as_ref()
            .is_some_and(|v| v.iter().any(|dns| !valid_ipv4(dns)))
    {
        return network_action(
            StatusCode::BAD_REQUEST,
            &state,
            false,
            "ip-apply",
            "Check IP address, subnet, gateway, and DNS values.",
            Some("validate"),
        );
    }
    let address = format!("{}/{}", ip, prefix);
    let gateway = body.gateway.unwrap_or_default();
    let dns = body.dns_servers.unwrap_or_default().join(" ");
    let ok = Command::new(NETWORK_MANAGER_BIN)
        .args([
            "connection",
            "modify",
            iface.as_str(),
            "ipv4.method",
            "manual",
            "ipv4.addresses",
            address.as_str(),
            "ipv4.gateway",
            gateway.as_str(),
            "ipv4.dns",
            dns.as_str(),
        ])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if ok {
        network_action(
            StatusCode::OK,
            &state,
            true,
            "ip-apply",
            &format!(
                "Network settings changed. Reconnect at http://{} and confirm within 90 seconds.",
                ip
            ),
            Some("confirm-reachable"),
        )
    } else {
        network_action(
            StatusCode::INTERNAL_SERVER_ERROR,
            &state,
            false,
            "ip-apply",
            "Manual IP settings could not be staged.",
            Some("apply"),
        )
    }
}

async fn ip_confirm(
    State(state): State<Arc<AppState>>,
    Json(body): Json<IpConfirmRequest>,
) -> (StatusCode, Json<NetworkActionResponse>) {
    let message = if body.token.as_deref().unwrap_or("").is_empty() {
        "Network settings confirmed."
    } else {
        "Network settings confirmed with browser token."
    };
    network_action(
        StatusCode::OK,
        &state,
        true,
        "ip-confirm",
        message,
        Some("confirmed"),
    )
}

async fn ip_rollback(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<NetworkActionResponse>) {
    network_action(
        StatusCode::OK,
        &state,
        true,
        "ip-rollback",
        "Settings rolled back.",
        Some("rolled-back"),
    )
}

async fn diagnostics_run(
    State(state): State<Arc<AppState>>,
    Json(body): Json<DiagnosticsRequest>,
) -> (StatusCode, Json<DiagnosticsResponse>) {
    let requested = body.tests.unwrap_or_else(|| {
        vec![
            "gateway".into(),
            "dns".into(),
            "internet".into(),
            "game-folders".into(),
            "lan-ai".into(),
        ]
    });
    let ns = network_state(&state);
    let mut results = Vec::new();
    for test in requested {
        match test.as_str() {
            "gateway" => results.push(diag(
                "Gateway",
                ns.active_connection.gateway.is_some(),
                if ns.active_connection.gateway.is_some() {
                    "Gateway reachable"
                } else {
                    "Gateway unreachable. Check cable, Wi-Fi, or DHCP."
                },
            )),
            "dns" => results.push(diag(
                "DNS",
                !ns.active_connection.dns_servers.is_empty(),
                if ns.active_connection.dns_servers.is_empty() {
                    "DNS failed. Internet names may not resolve."
                } else {
                    "DNS working"
                },
            )),
            "internet" => results.push(diag(
                "Internet",
                ns.active_connection.internet_reachable == Some(true),
                if ns.active_connection.internet_reachable == Some(true) {
                    "Internet reachable"
                } else {
                    "Internet unavailable"
                },
            )),
            "game-folders" => results.push(diag(
                "Game folders",
                ns.services.samba.state == "available",
                if ns.services.samba.state == "available" {
                    "Game folders available"
                } else {
                    "Game folders unavailable. Samba may be stopped."
                },
            )),
            "lan-ai" => results.push(diag(
                "LAN AI",
                ns.services.lan_inference.state == "available",
                if ns.services.lan_inference.state == "available" {
                    "LAN AI available"
                } else {
                    "LAN AI disabled"
                },
            )),
            _ => results.push(diag("Unknown", false, "Unknown diagnostic.")),
        }
    }
    let ok = results.iter().all(|r| r.ok || r.name == "LAN AI");
    (
        StatusCode::OK,
        Json(DiagnosticsResponse {
            ok,
            action: "network-diagnostics",
            results,
            state: ns,
        }),
    )
}

fn network_action(
    status: StatusCode,
    state: &AppState,
    ok: bool,
    action: &'static str,
    message: &str,
    stage: Option<&str>,
) -> (StatusCode, Json<NetworkActionResponse>) {
    (
        status,
        Json(NetworkActionResponse {
            ok,
            action,
            message: message.to_string(),
            stage: stage.map(str::to_string),
            state: network_state(state),
        }),
    )
}

fn diag(name: &str, ok: bool, message: &str) -> DiagnosticResult {
    DiagnosticResult {
        name: name.to_string(),
        ok,
        message: message.to_string(),
        detail: None,
    }
}

fn valid_ipv4(value: &str) -> bool {
    value.parse::<Ipv4Addr>().is_ok()
}

async fn pre_unlock(
    Json(body): Json<GuiPinUnlockRequest>,
) -> (StatusCode, Json<GuiPinActionResponse>) {
    if body.pin.is_empty() {
        return gui_pin_response(
            StatusCode::BAD_REQUEST,
            false,
            gui_pin_required(),
            "verify-pin",
            helper_exists(GUI_PIN_VERIFY_HELPER),
            "GUI PIN is required.",
        );
    }

    run_gui_pin_helper(
        GUI_PIN_VERIFY_HELPER,
        "verify-pin",
        &[body.pin.as_str()],
        "GUI PIN accepted.",
        "GUI PIN rejected.",
    )
}

async fn set_gui_pin_access(
    Json(body): Json<GuiPinAccessRequest>,
) -> (StatusCode, Json<GuiPinActionResponse>) {
    run_gui_pin_helper(
        GUI_PIN_ACCESS_HELPER,
        if body.pin_required {
            "require-gui-pin"
        } else {
            "disable-gui-pin"
        },
        &[if body.pin_required {
            "required"
        } else {
            "disabled"
        }],
        if body.pin_required {
            "GUI PIN is now required before Arcadia opens."
        } else {
            "GUI PIN gate is disabled; Arcadia opens directly."
        },
        "GUI PIN access setting failed.",
    )
}

async fn change_gui_pin(
    Json(body): Json<PinChangeRequest>,
) -> (StatusCode, Json<GuiPinActionResponse>) {
    if body.current_pin.is_empty() || body.new_pin.is_empty() {
        return gui_pin_response(
            StatusCode::BAD_REQUEST,
            false,
            gui_pin_required(),
            "change-pin",
            helper_exists(GUI_PIN_CHANGE_HELPER),
            "Current PIN and new PIN are required.",
        );
    }
    if body.new_pin.len() < 4 {
        return gui_pin_response(
            StatusCode::BAD_REQUEST,
            false,
            gui_pin_required(),
            "change-pin",
            helper_exists(GUI_PIN_CHANGE_HELPER),
            "New PIN is too short.",
        );
    }

    run_gui_pin_helper(
        GUI_PIN_CHANGE_HELPER,
        "change-pin",
        &[body.current_pin.as_str(), body.new_pin.as_str()],
        "GUI PIN changed.",
        "GUI PIN change failed.",
    )
}

async fn reset_gui_pin_default(
    Json(body): Json<PinResetRequest>,
) -> (StatusCode, Json<GuiPinActionResponse>) {
    if body.confirm != "RESET" {
        return gui_pin_response(
            StatusCode::BAD_REQUEST,
            false,
            gui_pin_required(),
            "reset-default-pin",
            helper_exists(GUI_PIN_RESET_HELPER),
            "Type RESET to restore the default GUI PIN.",
        );
    }

    run_gui_pin_helper(
        GUI_PIN_RESET_HELPER,
        "reset-default-pin",
        &[],
        "GUI PIN reset to the appliance default.",
        "GUI PIN reset failed.",
    )
}

fn run_gui_pin_helper(
    helper: &'static str,
    action: &'static str,
    secret_lines: &[&str],
    success_message: &str,
    failure_message: &str,
) -> (StatusCode, Json<GuiPinActionResponse>) {
    if !helper_exists(helper) {
        return gui_pin_response(
            StatusCode::NOT_IMPLEMENTED,
            false,
            gui_pin_required(),
            action,
            false,
            "GUI PIN helper is not installed on this unit yet.",
        );
    }

    let mut child = match Command::new(helper)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => {
            return gui_pin_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                false,
                gui_pin_required(),
                action,
                true,
                "Failed to start GUI PIN helper.",
            )
        }
    };

    if let Some(mut stdin) = child.stdin.take() {
        for line in secret_lines {
            let _ = stdin.write_all(line.as_bytes());
            let _ = stdin.write_all(b"\n");
        }
    }

    let ok = child.wait().map(|status| status.success()).unwrap_or(false);
    gui_pin_response(
        if ok {
            StatusCode::OK
        } else {
            StatusCode::FORBIDDEN
        },
        ok,
        gui_pin_required(),
        action,
        true,
        if ok { success_message } else { failure_message },
    )
}

fn gui_pin_response(
    status: StatusCode,
    ok: bool,
    pin_required: bool,
    action: &'static str,
    helper_present: bool,
    message: &str,
) -> (StatusCode, Json<GuiPinActionResponse>) {
    (
        status,
        Json(GuiPinActionResponse {
            ok,
            pin_required,
            action,
            helper_present,
            message: message.to_string(),
        }),
    )
}

async fn css() -> Response {
    asset(APP_CSS, "text/css; charset=utf-8")
}
async fn js() -> Response {
    asset(APP_JS, "application/javascript; charset=utf-8")
}
async fn not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "not found")
}

fn asset(body: &'static str, content_type: &'static str) -> Response {
    let mut response = body.into_response();
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    response
}

fn console_status(state: &AppState) -> ConsoleStatus {
    let storage = storage_status();
    let network = network_status();
    let (surfaces, samba) = surface_and_samba_status(&state.canonical_url, &network);
    let library = library_status(&storage);
    let hostname = hostname();
    ConsoleStatus {
        schema: "arcadia.home_state.v1",
        product: state.product.clone(),
        canonical_url: state.canonical_url.clone(),
        identity: IdentityStatus {
            product_name: state.product.clone(),
            hostname: hostname.clone(),
            local_domain: Some("home.arpa".to_string()),
            netbios_name: Some("HOMECONSOLE".to_string()),
            web_origin: state.canonical_url.trim_end_matches('/').to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        arcadia: ArcadiaStatus {
            service: service_state("arcadia.service"),
            version: env!("CARGO_PKG_VERSION"),
            mode: "unity-appliance-shell",
            ui: "top-header-left-launcher-focused-viewports",
        },
        runtime: runtime_status(state.started_unix),
        gui_pin: gui_pin_status(),
        surfaces,
        samba,
        network,
        local_ai: local_ai_status(),
        updates: updates_status(),
        library,
        storage,
        ui_contract: UiContract {
            schema: "arcadia.ui.contract.v6",
            button_variants: ["primary", "secondary", "danger"],
            composition: "top header, sidebar launcher, state-first operational viewport",
            modal: "confirmation/readback only; GUI PIN changes post to local root-owned helpers",
        },
    }
}

fn service_state(unit: &'static str) -> &'static str {
    let Ok(output) = Command::new(SYSTEMCTL_BIN)
        .args(["is-active", unit])
        .output()
    else {
        return "unknown";
    };
    let state = String::from_utf8_lossy(&output.stdout);
    match state.trim() {
        "active" => "running",
        "inactive" | "failed" => "stopped",
        "activating" => "starting",
        _ => "unknown",
    }
}

fn command_stdout(command: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(command).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn parse_global_ipv4_address(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        while let Some(part) = parts.next() {
            if part == "inet" {
                return parts
                    .next()
                    .and_then(|cidr| cidr.split('/').next())
                    .filter(|addr| !addr.is_empty())
                    .map(str::to_string);
            }
        }
        None
    })
}

fn network_status() -> NetworkStatus {
    let state = network_state_from_parts("HomeConsole", None);
    let online = state.active_connection.connection_type != "offline";
    NetworkStatus {
        online,
        active_type: state.active_connection.connection_type.clone(),
        connection_type: connection_label(&state.active_connection.connection_type).to_string(),
        ssid: state.wifi.connected_ssid.clone(),
        ip_address: state
            .active_connection
            .ip
            .clone()
            .unwrap_or_else(|| "—".to_string()),
        gateway: state.active_connection.gateway.clone(),
        dns_status: if state.active_connection.dns_servers.is_empty() {
            "Unknown".to_string()
        } else {
            "DNS working".to_string()
        },
        signal: state.wifi.signal_percent.map(|v| format!("{}%", v)),
        signal_percent: state.wifi.signal_percent,
        ethernet_speed_mbps: state.ethernet.speed_mbps,
        ethernet_available: state.ethernet.available,
        ethernet_connected: state.ethernet.connected,
        ethernet_mac_address: state.ethernet.mac_address.clone(),
        ethernet_dhcp: state.ethernet.dhcp,
        wifi_adapter_available: state.wifi.adapter_available,
        console_reachable: state.active_connection.lan_reachable,
        game_folders_reachable: state.services.samba.state == "available",
        samba_reachable: state.services.samba.state == "available",
        lan_ai_reachable: state.services.lan_inference.state == "available",
        internet_reachable: state.active_connection.internet_reachable,
    }
}

fn network_state(state: &AppState) -> NetworkState {
    network_state_from_parts(
        &state.product,
        Some(state.canonical_url.trim_end_matches('/')),
    )
}

fn network_state_from_parts(product: &str, canonical_url: Option<&str>) -> NetworkState {
    let hostname = hostname();
    let web_origin = canonical_url
        .map(str::to_string)
        .unwrap_or_else(|| format!("http://{}.home.arpa", hostname));
    let local_domain = format!("{}.home.arpa", hostname);
    let netbios = netbios_name(&hostname);
    let devices = nmcli_device_rows();
    let conns = nmcli_connection_rows();
    let saved = saved_wifi_networks(&conns);
    let scan = wifi_scan_results(&saved);
    let active = active_connection_from_nmcli(&devices);
    let dns_servers = dns_servers();
    let gateway = default_gateway();
    let ethernet = ethernet_state(&devices, &active, &dns_servers, gateway.clone());
    let wifi = wifi_state(&devices, &active, &saved, scan);
    let lan_reachable = active.ip.is_some() || gateway.is_some();
    let internet = lan_reachable.then(internet_reachable);
    let active_connection = ActiveConnection {
        connection_type: if active.kind == "wifi" {
            "wifi"
        } else if active.kind == "ethernet" {
            "ethernet"
        } else if lan_reachable {
            "limited"
        } else {
            "offline"
        }
        .to_string(),
        interface_name: active.interface_name.clone(),
        ip: active.ip.clone(),
        prefix_length: active.prefix_length,
        gateway,
        dns_servers: dns_servers.clone(),
        internet_reachable: internet,
        lan_reachable,
    };
    let services = network_services(&web_origin, active.ip.as_deref(), &hostname, &netbios);
    NetworkState {
        appliance: NetworkAppliance {
            product_name: product.to_string(),
            hostname,
            local_domain: Some(local_domain),
            netbios_name: Some(netbios),
            web_origin,
        },
        active_connection,
        ethernet,
        wifi,
        services,
    }
}

#[derive(Default, Clone)]
struct ActiveNetInfo {
    kind: String,
    interface_name: Option<String>,
    ip: Option<String>,
    prefix_length: Option<u8>,
    ssid: Option<String>,
    signal_percent: Option<u8>,
    security: Option<String>,
}

fn connection_label(kind: &str) -> &'static str {
    match kind {
        "ethernet" => "Ethernet",
        "wifi" => "Wi-Fi",
        "limited" => "Limited",
        "offline" => "Offline",
        _ => "Unknown",
    }
}

fn nmcli_device_rows() -> Vec<Vec<String>> {
    command_stdout(
        NETWORK_MANAGER_BIN,
        &[
            "-t",
            "-f",
            "DEVICE,TYPE,STATE,CONNECTION",
            "device",
            "status",
        ],
    )
    .map(|text| text.lines().map(split_nmcli_line).collect())
    .unwrap_or_default()
}

fn nmcli_connection_rows() -> Vec<Vec<String>> {
    command_stdout(
        NETWORK_MANAGER_BIN,
        &["-t", "-f", "NAME,TYPE,TIMESTAMP", "connection", "show"],
    )
    .map(|text| text.lines().map(split_nmcli_line).collect())
    .unwrap_or_default()
}

fn split_nmcli_line(line: &str) -> Vec<String> {
    line.split(':').map(|v| v.replace("\\:", ":")).collect()
}

fn active_connection_from_nmcli(devices: &[Vec<String>]) -> ActiveNetInfo {
    for parts in devices {
        let dev = parts.get(0).cloned().unwrap_or_default();
        let kind = parts.get(1).cloned().unwrap_or_default();
        let state = parts.get(2).cloned().unwrap_or_default();
        if state != "connected" || dev == "lo" {
            continue;
        }
        let ip = interface_ipv4(&dev);
        let mut info = ActiveNetInfo {
            kind: if kind == "wifi" {
                "wifi".into()
            } else if kind == "ethernet" {
                "ethernet".into()
            } else {
                "unknown".into()
            },
            interface_name: Some(dev.clone()),
            prefix_length: interface_prefix(&dev),
            ip,
            ..Default::default()
        };
        if kind == "wifi" {
            info.ssid = parts
                .get(3)
                .cloned()
                .filter(|v| !v.is_empty() && v != "--")
                .or_else(|| wifi_ssid(&dev));
            info.signal_percent = wifi_signal_for_ssid(info.ssid.as_deref());
            info.security = wifi_security_for_ssid(info.ssid.as_deref());
        }
        return info;
    }
    ActiveNetInfo::default()
}

fn interface_ipv4(dev: &str) -> Option<String> {
    command_stdout(
        "ip",
        &["-4", "-o", "addr", "show", "dev", dev, "scope", "global"],
    )
    .and_then(|text| parse_global_ipv4_address(&text))
}

fn interface_prefix(dev: &str) -> Option<u8> {
    command_stdout(
        "ip",
        &["-4", "-o", "addr", "show", "dev", dev, "scope", "global"],
    )
    .and_then(|text| {
        text.lines().find_map(|line| {
            line.split_whitespace()
                .collect::<Vec<_>>()
                .windows(2)
                .find_map(|w| {
                    if w[0] == "inet" {
                        w[1].split('/').nth(1)?.parse().ok()
                    } else {
                        None
                    }
                })
        })
    })
}

fn default_gateway() -> Option<String> {
    command_stdout("ip", &["route", "show", "default"]).and_then(|text| {
        let parts: Vec<&str> = text.split_whitespace().collect();
        parts.windows(2).find_map(|w| {
            if w[0] == "via" {
                Some(w[1].to_string())
            } else {
                None
            }
        })
    })
}

fn dns_servers() -> Vec<String> {
    fs::read_to_string("/etc/resolv.conf")
        .unwrap_or_default()
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            if parts.next()? == "nameserver" {
                parts.next().map(str::to_string)
            } else {
                None
            }
        })
        .filter(|v| valid_ipv4(v))
        .collect()
}

fn ethernet_state(
    devices: &[Vec<String>],
    active: &ActiveNetInfo,
    dns: &[String],
    gateway: Option<String>,
) -> EthernetState {
    let iface = devices
        .iter()
        .find(|p| p.get(1).map(|v| v == "ethernet").unwrap_or(false))
        .and_then(|p| p.first())
        .cloned()
        .or_else(ethernet_interface_name);
    let connected = iface
        .as_ref()
        .map(|name| active.interface_name.as_deref() == Some(name.as_str()) || carrier_up(name))
        .unwrap_or(false);
    let ip = iface.as_ref().and_then(|name| interface_ipv4(name));
    EthernetState {
        available: iface.is_some(),
        connected,
        interface_name: iface.clone(),
        mac_address: iface
            .as_ref()
            .and_then(|name| fs::read_to_string(format!("/sys/class/net/{}/address", name)).ok())
            .map(|v| v.trim().to_string()),
        speed_mbps: iface.as_ref().and_then(|name| read_speed_mbps(name)),
        ip,
        dhcp: true,
        gateway,
        dns_servers: dns.to_vec(),
    }
}

fn wifi_state(
    _devices: &[Vec<String>],
    active: &ActiveNetInfo,
    saved: &[SavedWifiNetwork],
    mut scan_results: Vec<WifiScanResult>,
) -> WifiState {
    let adapter = wifi_adapter_name();
    for row in &mut scan_results {
        if active.ssid.as_deref() == Some(row.ssid.as_str()) {
            row.connected = true;
        }
        if saved.iter().any(|s| s.ssid == row.ssid) {
            row.saved = true;
        }
    }
    WifiState {
        adapter_available: adapter.is_some(),
        enabled: wifi_enabled(),
        scanning: false,
        connected_ssid: active.ssid.clone(),
        signal_percent: active.signal_percent,
        security: active.security.clone(),
        saved_networks: saved.to_vec(),
        scan_results,
    }
}

fn saved_wifi_networks(conns: &[Vec<String>]) -> Vec<SavedWifiNetwork> {
    conns
        .iter()
        .filter(|p| {
            p.get(1)
                .map(|v| v.contains("wireless") || v == "wifi")
                .unwrap_or(false)
        })
        .filter_map(|p| p.first().cloned())
        .map(|ssid| SavedWifiNetwork {
            ssid,
            security: None,
            last_connected_at: None,
        })
        .collect()
}

fn wifi_scan_results(saved: &[SavedWifiNetwork]) -> Vec<WifiScanResult> {
    let Some(text) = command_stdout(
        NETWORK_MANAGER_BIN,
        &[
            "-t",
            "-f",
            "SSID,BSSID,SIGNAL,SECURITY,IN-USE",
            "device",
            "wifi",
            "list",
        ],
    ) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| {
            let p = split_nmcli_line(line);
            let ssid = p.first()?.trim().to_string();
            if ssid.is_empty() {
                return None;
            }
            let signal = p
                .get(2)
                .and_then(|v| v.parse::<u8>().ok())
                .unwrap_or(0)
                .min(100);
            let security = normalize_wifi_security(p.get(3).map(String::as_str).unwrap_or(""));
            Some(WifiScanResult {
                ssid: ssid.clone(),
                bssid: p.get(1).cloned().filter(|v| !v.is_empty()),
                signal_percent: signal,
                security,
                saved: saved.iter().any(|s| s.ssid == ssid),
                connected: p.get(4).map(|v| v == "*").unwrap_or(false),
            })
        })
        .collect()
}

fn normalize_wifi_security(raw: &str) -> String {
    let value = raw.to_ascii_lowercase();
    if value.trim().is_empty() || value == "--" {
        "open".into()
    } else if value.contains("wpa3") {
        "wpa3".into()
    } else if value.contains("wpa2") && value.contains("wpa1") {
        "wpa-wpa2".into()
    } else if value.contains("wpa2") || value.contains("wpa") {
        "wpa2".into()
    } else {
        "unknown".into()
    }
}

fn wifi_enabled() -> bool {
    command_stdout(NETWORK_MANAGER_BIN, &["radio", "wifi"])
        .map(|v| v.trim() == "enabled")
        .unwrap_or_else(|| wifi_adapter_name().is_some())
}

fn wifi_ssid(dev: &str) -> Option<String> {
    command_stdout("iw", &["dev", dev, "link"]).and_then(|text| {
        text.lines()
            .find_map(|line| line.trim().strip_prefix("SSID: ").map(str::to_string))
    })
}

fn wifi_signal_for_ssid(ssid: Option<&str>) -> Option<u8> {
    let ssid = ssid?;
    wifi_scan_results(&[])
        .into_iter()
        .find(|row| row.ssid == ssid)
        .map(|row| row.signal_percent)
}

fn wifi_security_for_ssid(ssid: Option<&str>) -> Option<String> {
    let ssid = ssid?;
    wifi_scan_results(&[])
        .into_iter()
        .find(|row| row.ssid == ssid)
        .map(|row| row.security)
}

fn ethernet_interface_name() -> Option<String> {
    fs::read_dir("/sys/class/net")
        .ok()?
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .find(|n| n != "lo" && !n.starts_with("wl") && !n.starts_with("wifi"))
}

fn carrier_up(name: &str) -> bool {
    fs::read_to_string(format!("/sys/class/net/{}/carrier", name))
        .map(|v| v.trim() == "1")
        .unwrap_or(false)
}
fn read_speed_mbps(name: &str) -> Option<u64> {
    fs::read_to_string(format!("/sys/class/net/{}/speed", name))
        .ok()?
        .trim()
        .parse()
        .ok()
}

fn network_services(
    web_origin: &str,
    ip: Option<&str>,
    hostname: &str,
    netbios: &str,
) -> NetworkServices {
    let samba_ok = samba_available() && Path::new(GAMES_ROOT).exists();
    let share = "games";
    let mut urls = vec![web_origin.to_string()];
    if let Some(ip) = ip {
        urls.push(format!("http://{}", ip));
    }
    let share_row = SambaShareStatus {
        name: share.to_string(),
        purpose: "games".to_string(),
        windows_unc: samba_ok.then(|| format!(r"\\{}\{}", netbios, share)),
        windows_unc_by_ip: samba_ok
            .then(|| ip.map(|addr| format!(r"\\{}\{}", addr, share)))
            .flatten(),
        smb_url: samba_ok.then(|| format!("smb://{}/{}", hostname, share)),
        smb_url_by_ip: samba_ok
            .then(|| ip.map(|addr| format!("smb://{}/{}", addr, share)))
            .flatten(),
    };
    let lan_ai = tcp_port_listening(LAN_INFERENCE_PORT);
    NetworkServices {
        web_console: WebConsoleService {
            state: "available".to_string(),
            urls,
        },
        samba: SambaServiceState {
            state: if samba_ok {
                "available"
            } else if SAMBA_SERVICE_NAMES
                .iter()
                .any(|unit| service_state(unit) == "unknown")
            {
                "unknown"
            } else {
                "disabled"
            }
            .to_string(),
            shares: samba_ok.then(|| vec![share_row]).unwrap_or_default(),
        },
        lan_inference: LanInferenceService {
            state: if lan_ai { "available" } else { "disabled" }.to_string(),
            port: LAN_INFERENCE_PORT,
            urls: lan_ai
                .then(|| vec![format!("{}:{}", web_origin, LAN_INFERENCE_PORT)])
                .unwrap_or_default(),
        },
        ssh: SshService {
            state: if service_state("sshd.service") == "running" || tcp_port_listening(22) {
                "available"
            } else {
                "disabled"
            }
            .to_string(),
            port: 22,
        },
    }
}

fn netbios_name(hostname: &str) -> String {
    let cleaned: String = hostname
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect::<String>()
        .to_ascii_uppercase();
    if cleaned.is_empty() {
        "HOMECONSOLE".to_string()
    } else {
        cleaned.chars().take(15).collect()
    }
}

fn internet_reachable() -> bool {
    let Ok(addr) = "1.1.1.1:53".parse() else {
        return false;
    };
    TcpStream::connect_timeout(&addr, Duration::from_millis(180)).is_ok()
}

fn tcp_port_listening(port: u16) -> bool {
    let needle = format!(":{:04X}", port);
    fs::read_to_string("/proc/net/tcp")
        .map(|text| {
            text.lines()
                .skip(1)
                .any(|line| line.contains(&needle) && line.split_whitespace().nth(3) == Some("0A"))
        })
        .unwrap_or(false)
}

fn library_status(storage: &StorageStatus) -> LibraryStatus {
    let game_files = current_game_files();
    let detected_games = game_files.len() as u64;
    let gamescope_entries = count_gamescope_entries();
    let manifest = load_sync_manifest();
    let mut unsynced_added = 0u64;
    let mut unsynced_changed = 0u64;
    let mut unsynced_removed = 0u64;
    let mut total_synced_entries = 0u64;
    let mut artwork_complete = 0u64;
    let mut artwork_missing = 0u64;
    let sync_state;
    if let Some(entries) = manifest {
        let mut by_path = HashMap::new();
        for entry in entries {
            let key = entry
                .normalized_rom_path
                .clone()
                .or(entry.rom_path.clone())
                .unwrap_or_default();
            if key.is_empty() {
                continue;
            }
            if entry.last_synced_at.is_some() || entry.gamescope_entry_id.is_some() {
                total_synced_entries += 1;
            }
            match entry.artwork_status.as_deref() {
                Some("complete") => artwork_complete += 1,
                Some("missing") => artwork_missing += 1,
                _ => {}
            }
            by_path.insert(normalize_path(&key), entry);
        }
        for file in &game_files {
            match by_path.remove(&file.normalized_rom_path) {
                Some(entry) => {
                    if entry.size_bytes != 0 && entry.size_bytes != file.size_bytes {
                        unsynced_changed += 1;
                    } else if entry.mtime_ms != 0 && entry.mtime_ms != file.mtime_ms {
                        unsynced_changed += 1;
                    }
                }
                None => unsynced_added += 1,
            }
        }
        unsynced_removed = by_path.len() as u64;
        sync_state = if unsynced_added + unsynced_changed + unsynced_removed > 0 {
            "idle"
        } else {
            "idle"
        }
        .to_string();
    } else {
        total_synced_entries = gamescope_entries.min(detected_games);
        artwork_complete = storage.artwork.files.min(detected_games);
        artwork_missing = detected_games.saturating_sub(artwork_complete);
        sync_state = if detected_games == 0 {
            "idle"
        } else {
            "unknown"
        }
        .to_string();
    }
    let sync_needed =
        sync_state != "unknown" && unsynced_added + unsynced_changed + unsynced_removed > 0;
    let last_sync_state = latest_sync_summary().unwrap_or_else(|| {
        if sync_state == "unknown" {
            "unknown".to_string()
        } else if total_synced_entries > 0 {
            "success".to_string()
        } else {
            "never".to_string()
        }
    });
    let last_sync = match last_sync_state.as_str() {
        "success" => "Today".to_string(),
        "error" => "Failed".to_string(),
        "running" => "Running".to_string(),
        "unknown" => "Unknown".to_string(),
        _ if total_synced_entries > 0 => "Synced".to_string(),
        _ => "Never".to_string(),
    };
    let artwork_status = if artwork_complete > 0 && artwork_missing > 0 {
        format!(
            "{} complete · {} missing",
            artwork_complete, artwork_missing
        )
    } else if artwork_complete > 0 {
        format!("{} complete", artwork_complete)
    } else if sync_state == "unknown" {
        "Unknown".to_string()
    } else if detected_games > 0 {
        "0 complete".to_string()
    } else {
        "No artwork".to_string()
    };
    LibraryStatus {
        detected_games,
        detected_files: detected_games,
        gamescope_entries,
        first_sync_completed: total_synced_entries > 0,
        last_sync,
        last_sync_at: None,
        last_sync_state,
        artwork_status,
        artwork_complete,
        artwork_missing,
        sync_needed,
        sync_state,
        unsynced_added,
        unsynced_changed,
        unsynced_removed,
        total_detected_games: detected_games,
        total_synced_entries,
    }
}

fn current_game_files() -> Vec<GameFileState> {
    let mut files = Vec::new();
    for system in GAME_SYSTEMS {
        collect_game_files(
            Path::new(GAMES_ROOT).join(system).as_path(),
            system,
            &mut files,
            0,
        );
    }
    files
}

fn collect_game_files(path: &Path, _platform: &str, files: &mut Vec<GameFileState>, depth: usize) {
    if depth > 6 {
        return;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if metadata.is_dir() {
            collect_game_files(&p, _platform, files, depth + 1);
        } else if metadata.is_file() {
            let normalized_rom_path = normalize_path(&p.to_string_lossy());
            let mtime_ms = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            files.push(GameFileState {
                normalized_rom_path,
                size_bytes: metadata.len(),
                mtime_ms,
            });
        }
    }
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/").to_ascii_lowercase()
}

fn load_sync_manifest() -> Option<Vec<SyncManifestEntry>> {
    for path in SYNC_MANIFEST_PATHS {
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        if let Ok(doc) = serde_json::from_str::<SyncManifestDoc>(&text) {
            return Some(doc.entries);
        }
        if let Ok(entries) = serde_json::from_str::<Vec<SyncManifestEntry>>(&text) {
            return Some(entries);
        }
    }
    None
}

fn count_gamescope_entries() -> u64 {
    let roots = [
        "/home/owner/.local/share/applications",
        "/home/owner/.steam/steam/userdata",
    ];
    roots
        .iter()
        .map(|root| count_files_with_extension(Path::new(root), "desktop", 5))
        .sum()
}

fn count_files_with_extension(path: &Path, ext: &str, depth: usize) -> u64 {
    if depth == 0 {
        return 0;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    let mut count = 0;
    for entry in entries.flatten() {
        let p = entry.path();
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if metadata.is_dir() {
            count += count_files_with_extension(&p, ext, depth - 1);
        } else if metadata.is_file()
            && p.extension()
                .and_then(|v| v.to_str())
                .map(|v| v.eq_ignore_ascii_case(ext))
                .unwrap_or(false)
        {
            count += 1;
        }
    }
    count
}

fn latest_sync_summary() -> Option<String> {
    let paths = [
        "/var/lib/harmonia/receipts/game-sync-latest/run.json",
        "/var/lib/harmonia/receipts/homeconsole-sync-latest/run.json",
    ];
    for path in paths {
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        if text.contains("\"ok\":true") || text.contains("\"ok\": true") {
            return Some("success".to_string());
        }
        if text.contains("\"ok\":false") || text.contains("\"ok\": false") {
            return Some("error".to_string());
        }
    }
    None
}

fn local_ai_state(state: &AppState) -> LocalAIState {
    let status = local_ai_status();
    let installed = status.available_models.clone();
    let runtime_installed = helper_exists(LLAMA_SERVER_BIN)
        || helper_exists(LLAMA_CPP_BIN)
        || command_stdout("which", &["llama-server"]).is_some();
    let inference_listening = tcp_port_listening(LAN_INFERENCE_PORT);
    let server_running =
        inference_listening || command_stdout("pgrep", &["-af", "llama-server"]).is_some();
    let storage = storage_status();
    let free_storage = storage.free_bytes;
    let model_storage = storage.ai_models.bytes;
    let endpoint = format!(
        "{}:{}",
        state.canonical_url.trim_end_matches('/'),
        LAN_INFERENCE_PORT
    );
    let selected = selected_model_id().or_else(|| status.selected_model_id.clone());
    let selected_name = selected
        .as_ref()
        .and_then(|id| {
            installed
                .iter()
                .find(|m| &m.id == id)
                .map(|m| m.name.clone())
        })
        .or(status.selected_model_name.clone());
    LocalAIState {
        runtime: AIRuntimeState {
            installed: runtime_installed,
            name: "llama.cpp".to_string(),
            version: llama_version(),
            latest_version: None,
            update_state: "idle".to_string(),
            server_state: if server_running { "running" } else { "stopped" }.to_string(),
            error: None,
        },
        loaded_model: AILoadedModelState {
            load_state: status.load_state.clone(),
            selected_model_id: selected,
            selected_model_name: selected_name,
            loaded_model_id: status.loaded_model_id.clone(),
            loaded_model_name: status.loaded_model_name.clone(),
            error: None,
        },
        installed_models: installed.clone(),
        recommended_models: recommended_ai_models(&installed),
        downloads: active_ai_downloads(),
        inference: InferenceState {
            enabled: inference_listening,
            lan_access_enabled: inference_listening,
            host: if inference_listening {
                "lan"
            } else {
                "localhost"
            }
            .to_string(),
            port: LAN_INFERENCE_PORT,
            endpoint_urls: if inference_listening {
                vec![endpoint]
            } else {
                Vec::new()
            },
            api_mode: Some("openai-compatible".to_string()),
            request_count: None,
            last_request_at: None,
        },
        hardware: AIHardwareState {
            gpu_memory_used_bytes: status.gpu_memory_used_bytes,
            gpu_memory_total_bytes: status.gpu_memory_total_bytes,
            model_storage_bytes: model_storage,
            free_storage_bytes: free_storage,
        },
        activity: AIActivityState {
            current_operation: if inference_listening {
                "serving inference"
            } else if server_running {
                "runtime running"
            } else {
                "idle"
            }
            .to_string(),
            last_error: None,
            runtime_update_log: redacted_log(
                "/var/lib/harmonia/receipts/local-ai-runtime-latest/events.jsonl",
            ),
            model_download_log: redacted_log("/var/lib/arcadia/local-ai-download.log"),
            model_load_log: redacted_log("/var/log/arcadia-local-ai.log"),
            inference_server_log: redacted_log("/var/log/llama-server.log"),
        },
    }
}

fn selected_model_id() -> Option<String> {
    let text = fs::read_to_string(LOCAL_AI_STATE_PATH).ok()?;
    serde_json::from_str::<serde_json::Value>(&text)
        .ok()?
        .get("selectedModelId")?
        .as_str()
        .map(str::to_string)
}

fn llama_version() -> Option<String> {
    command_stdout(LLAMA_SERVER_BIN, &["--version"])
        .or_else(|| command_stdout(LLAMA_CPP_BIN, &["--version"]))
        .and_then(|text| text.lines().next().map(|v| v.trim().to_string()))
}

fn recommended_ai_models(installed: &[LocalAiModelStatus]) -> Vec<AIRecommendedModel> {
    let inharmonia_installed = installed.iter().any(|m| m.is_inharmonia);
    vec![AIRecommendedModel {
        id: "inharmonia".to_string(),
        name: "Inharmonia".to_string(),
        description: "Balanced local assistant for Arcadia.".to_string(),
        source: "catalog".to_string(),
        repo_id: None,
        filename: None,
        size_bytes: None,
        estimated_vram_bytes: None,
        recommended_use: Some("balanced".to_string()),
        is_inharmonia: true,
        install_state: if inharmonia_installed {
            "installed"
        } else {
            "available"
        }
        .to_string(),
    }]
}

fn active_ai_downloads() -> Vec<AIDownloadState> {
    Vec::new()
}

fn redacted_log(path: &str) -> String {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .rev()
        .take(40)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n")
        .replace("token", "[REDACTED]")
        .replace("password", "[REDACTED]")
        .replace("api_key", "[REDACTED]")
}

fn quantization_from_filename(filename: &str) -> Option<String> {
    let upper = filename.to_ascii_uppercase();
    [
        "Q2_K",
        "Q3_K_M",
        "Q4_K_M",
        "Q5_K_M",
        "Q6_K",
        "Q8_0",
        "UD_Q3_K_M",
    ]
    .iter()
    .find(|q| upper.contains(**q))
    .map(|q| q.to_string())
}

fn model_source_from_path(filename: &str, path: &Path) -> String {
    if filename.to_ascii_lowercase().contains("inharmonia") {
        "bundled".to_string()
    } else if path.to_string_lossy().contains("huggingface") {
        "huggingface".to_string()
    } else {
        "manual".to_string()
    }
}

fn repo_id_from_path(path: &Path) -> Option<String> {
    let text = path.to_string_lossy();
    let marker = "models--";
    let start = text.find(marker)? + marker.len();
    let rest = &text[start..];
    let repo = rest.split('/').next()?.replace("--", "/");
    Some(repo)
}

fn local_ai_status() -> LocalAiStatus {
    let mut available_models = local_ai_available_models();
    let loaded_model = command_stdout("pgrep", &["-af", "llama|ollama|vllm"])
        .and_then(|text| text.lines().next().map(str::to_string))
        .map(|line| {
            line.split_whitespace()
                .find(|part| {
                    part.ends_with(".gguf")
                        || part.ends_with(".safetensors")
                        || part.ends_with(".onnx")
                })
                .map(|part| {
                    Path::new(part)
                        .file_name()
                        .and_then(|v| v.to_str())
                        .unwrap_or(part)
                        .to_string()
                })
                .unwrap_or_else(|| "Local AI runtime".to_string())
        });
    if let Some(loaded) = &loaded_model {
        if !available_models
            .iter()
            .any(|model| model.filename == *loaded)
        {
            available_models.insert(
                0,
                LocalAiModelStatus {
                    id: model_id(loaded),
                    name: friendly_model_name(loaded),
                    filename: loaded.clone(),
                    source: "manual".to_string(),
                    repo_id: None,
                    size_bytes: 0,
                    size: "Unknown".to_string(),
                    quantization: quantization_from_filename(loaded),
                    estimated_vram_bytes: None,
                    recommended_use: None,
                    installed_at: None,
                    is_recommended: loaded.to_ascii_lowercase().contains("inharmonia"),
                    is_inharmonia: loaded.to_ascii_lowercase().contains("inharmonia"),
                },
            );
        }
    }
    let selected = loaded_model
        .as_ref()
        .and_then(|loaded| {
            available_models
                .iter()
                .find(|model| model.filename == *loaded)
        })
        .or_else(|| available_models.first());
    let load_state = if loaded_model.is_some() {
        "hot"
    } else if selected.is_some() {
        "cold"
    } else {
        "unloaded"
    }
    .to_string();
    let (gpu_used, gpu_total) = gpu_memory_bytes();
    let lan_inference_enabled = tcp_port_listening(LAN_INFERENCE_PORT);
    LocalAiStatus {
        load_state,
        selected_model_id: selected.map(|model| model.id.clone()),
        selected_model_name: selected.map(|model| model.name.clone()),
        loaded_model_id: loaded_model.as_ref().map(|name| model_id(name)),
        loaded_model_name: loaded_model.as_ref().map(|name| friendly_model_name(name)),
        loaded_model,
        available_models,
        gpu_memory: match (gpu_used, gpu_total) {
            (Some(used), Some(total)) => {
                Some(format!("{} / {}", human_size(used), human_size(total)))
            }
            _ => None,
        },
        gpu_memory_used_bytes: gpu_used,
        gpu_memory_total_bytes: gpu_total,
        lan_inference_enabled,
        lan_inference_port: lan_inference_enabled.then_some(LAN_INFERENCE_PORT),
    }
}

fn local_ai_available_models() -> Vec<LocalAiModelStatus> {
    let mut models = Vec::new();
    for root in MODEL_SCAN_ROOTS {
        let mut found = Vec::new();
        collect_ai_models(Path::new(root), &mut found, 0);
        for (size, filename, _path) in found {
            if !filename.to_ascii_lowercase().ends_with(".gguf") {
                continue;
            }
            models.push(LocalAiModelStatus {
                id: model_id(&filename),
                name: friendly_model_name(&filename),
                filename: filename.clone(),
                source: model_source_from_path(&filename, &_path),
                repo_id: repo_id_from_path(&_path),
                size_bytes: size,
                size: human_size(size),
                quantization: quantization_from_filename(&filename),
                estimated_vram_bytes: Some(size.saturating_add(size / 5)),
                recommended_use: Some(if size < 3_000_000_000 {
                    "fast"
                } else if size < 6_000_000_000 {
                    "balanced"
                } else {
                    "quality"
                }),
                installed_at: None,
                is_recommended: filename.to_ascii_lowercase().contains("inharmonia"),
                is_inharmonia: filename.to_ascii_lowercase().contains("inharmonia"),
            });
        }
    }
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

fn model_id(filename: &str) -> String {
    filename
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn gpu_memory_bytes() -> (Option<u64>, Option<u64>) {
    let Some(text) = command_stdout(
        "nvidia-smi",
        &[
            "--query-gpu=memory.used,memory.total",
            "--format=csv,noheader,nounits",
        ],
    ) else {
        return (None, None);
    };
    let first = text.lines().next().unwrap_or_default();
    let mut parts = first
        .split(',')
        .map(|part| part.trim().parse::<u64>().ok().map(|mib| mib * 1024 * 1024));
    (parts.next().flatten(), parts.next().flatten())
}

fn hostname() -> String {
    command_stdout("hostname", &[]).unwrap_or_else(|| "homeconsole".to_string())
}

fn wifi_adapter_name() -> Option<String> {
    command_stdout("iw", &["dev"])
        .and_then(|text| {
            text.lines()
                .find_map(|line| line.trim().strip_prefix("Interface ").map(str::to_string))
        })
        .or_else(|| {
            fs::read_dir("/sys/class/net")
                .ok()?
                .flatten()
                .map(|e| e.file_name().to_string_lossy().to_string())
                .find(|n| n.starts_with("wl"))
        })
}

fn samba_available() -> bool {
    SAMBA_SERVICE_NAMES
        .iter()
        .any(|unit| service_state(unit) == "running")
        || tcp_port_listening(445)
}

fn surface_and_samba_status(
    canonical_url: &str,
    network: &NetworkStatus,
) -> (SurfaceStatus, SambaStatus) {
    let netbios = "HOMECONSOLE".to_string();
    let host = hostname();
    let share = "games".to_string();
    let samba_ok = samba_available() && Path::new(GAMES_ROOT).exists();
    let windows_unc = samba_ok.then(|| format!(r"\\{}\{}", netbios, share));
    let windows_unc_by_ip = (samba_ok && network.ip_address != "—")
        .then(|| format!(r"\\{}\{}", network.ip_address, share));
    let smb_url = samba_ok.then(|| format!("smb://{}/{}", host, share));
    let share_status = SambaShareStatus {
        name: share.clone(),
        purpose: "games".to_string(),
        windows_unc: windows_unc.clone(),
        windows_unc_by_ip: windows_unc_by_ip.clone(),
        smb_url: smb_url.clone(),
        smb_url_by_ip: (samba_ok && network.ip_address != "—")
            .then(|| format!("smb://{}/{}", network.ip_address, share)),
    };
    let shares = if samba_ok {
        vec![share_status]
    } else {
        Vec::new()
    };
    (
        SurfaceStatus {
            http: canonical_url.trim_end_matches('/').to_string(),
            mdns: format!("{}.local", host),
            smb: netbios,
            windows_unc,
            windows_unc_by_ip,
            smb_url,
            smb_url_by_ip: (samba_ok && network.ip_address != "—")
                .then(|| format!("smb://{}/{}", network.ip_address, share)),
        },
        SambaStatus {
            state: if samba_ok {
                "available"
            } else if SAMBA_SERVICE_NAMES
                .iter()
                .any(|unit| service_state(unit) == "unknown")
            {
                "unknown"
            } else {
                "disabled"
            }
            .to_string(),
            shares,
        },
    )
}

fn updates_status() -> UpdatesStatus {
    let current = env!("CARGO_PKG_VERSION").to_string();
    let deployed_sha_meta = fs::metadata("/var/lib/harmonia/state/arcadia.sha").ok();
    let deploy_meta = fs::metadata("/var/lib/harmonia/receipts/arcadia-latest/run.json").ok();
    let check_meta = fs::metadata("/var/lib/harmonia/receipts/arcadia-check-latest/run.json").ok();
    let deploy_is_fresher = match (&deploy_meta, &check_meta) {
        (Some(deploy), Some(check)) => deploy.modified().ok() >= check.modified().ok(),
        (Some(_), None) => true,
        _ => false,
    };
    let deployed_ok = fs::read_to_string("/var/lib/harmonia/receipts/arcadia-latest/run.json")
        .map(|text| text.contains("\"ok\":true") || text.contains("\"ok\": true"))
        .unwrap_or(false);
    let state = if deployed_sha_meta.is_some() && deployed_ok && deploy_is_fresher {
        "current"
    } else if let Ok(text) =
        fs::read_to_string("/var/lib/harmonia/receipts/arcadia-check-latest/run.json")
    {
        if text.contains("\"update_available\":true") || text.contains("\"update_available\": true")
        {
            "available"
        } else if text.contains("\"ok\":false") || text.contains("\"ok\": false") {
            "error"
        } else {
            "current"
        }
    } else {
        "unknown"
    };
    UpdatesStatus {
        state: state.to_string(),
        current_version: current,
        available_version: None,
    }
}

fn runtime_status(started_unix: u64) -> RuntimeStatus {
    let machine_uptime_seconds = fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|uptime| uptime.split_whitespace().next().map(str::to_string))
        .and_then(|seconds| seconds.split('.').next().unwrap_or("0").parse::<u64>().ok())
        .unwrap_or(0);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(started_unix);
    let arcadia_uptime_seconds = now.saturating_sub(started_unix);
    RuntimeStatus {
        machine_uptime: format_duration(machine_uptime_seconds),
        machine_uptime_seconds,
        arcadia_uptime: format_duration(arcadia_uptime_seconds),
        arcadia_uptime_seconds,
    }
}

fn format_duration(total_seconds: u64) -> String {
    let days = total_seconds / 86_400;
    let hours = (total_seconds % 86_400) / 3_600;
    let minutes = (total_seconds % 3_600) / 60;
    if days > 0 {
        format!("{days}d {hours}h {minutes}m")
    } else if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    }
}

fn gui_pin_status() -> GuiPinStatus {
    let reset_helper_present = helper_exists(GUI_PIN_RESET_HELPER);
    GuiPinStatus {
        pin_required: gui_pin_required(),
        state_path: GUI_PIN_STATE_PATH,
        access_helper_present: helper_exists(GUI_PIN_ACCESS_HELPER),
        pin_change_helper_present: helper_exists(GUI_PIN_CHANGE_HELPER),
        pin_reset_helper_present: reset_helper_present,
        pin_storage: "keyman-redacted",
        default_reset_available: reset_helper_present,
    }
}

fn gui_pin_required() -> bool {
    fs::read_to_string(GUI_PIN_STATE_PATH)
        .map(|state| {
            let normalized = state.to_ascii_lowercase();
            normalized.contains("pin_required=true")
                || normalized.contains("pin_required: true")
                || normalized.contains("\"pin_required\":true")
                || normalized.trim() == "required"
                || normalized.trim() == "true"
        })
        .unwrap_or(false)
}

fn helper_exists(path: &str) -> bool {
    Path::new(path).exists()
}

fn storage_status() -> StorageStatus {
    storage_scan()
}

fn storage_scan() -> StorageStatus {
    let scan_started = SystemTime::now();
    let network = network_status();
    let registry = storage_registry(&network);
    let volume = root_volume();
    let total_bytes = volume.total_bytes;
    let used_bytes = volume.used_bytes;
    let free_bytes = volume.free_bytes;
    let percent_used = percent(used_bytes, total_bytes);
    let (health, header_state, header_class) = storage_health(percent_used);

    let game_folders = registry
        .categories
        .games
        .roots
        .iter()
        .map(game_folder_storage)
        .collect::<Vec<_>>();
    let games_bytes = game_folders.iter().map(|f| f.bytes).sum::<u64>();
    let games_files = game_folders.iter().map(|f| f.file_count).sum::<u64>();

    let artwork_stores = registry
        .categories
        .artwork
        .roots
        .iter()
        .map(|r| folder_storage(&r.id, &r.display_name, &r.path))
        .collect::<Vec<_>>();
    let artwork_bytes = artwork_stores.iter().map(|f| f.bytes).sum::<u64>();
    let artwork_files = artwork_stores.iter().map(|f| f.file_count).sum::<u64>();

    let ai_model_files = ai_model_files();
    let ai_model_bytes = ai_model_files.iter().map(|m| m.bytes).sum::<u64>();

    let update_stores = registry
        .categories
        .updates
        .roots
        .iter()
        .map(|r| folder_storage(&r.id, &r.display_name, &r.path))
        .collect::<Vec<_>>();
    let updates_bytes = update_stores.iter().map(|f| f.bytes).sum::<u64>();
    let updates_files = update_stores.iter().map(|f| f.file_count).sum::<u64>();

    let log_stores = registry
        .categories
        .logs
        .roots
        .iter()
        .map(|r| folder_storage(&r.id, &r.display_name, &r.path))
        .collect::<Vec<_>>();
    let logs_bytes = log_stores.iter().map(|f| f.bytes).sum::<u64>();
    let logs_files = log_stores.iter().map(|f| f.file_count).sum::<u64>();

    let temp_stores = registry
        .categories
        .temporary
        .roots
        .iter()
        .map(|r| folder_storage(&r.id, &r.display_name, &r.path))
        .collect::<Vec<_>>();
    let temporary_bytes = temp_stores.iter().map(|f| f.bytes).sum::<u64>();
    let temporary_files = temp_stores.iter().map(|f| f.file_count).sum::<u64>();

    let system_stores = registry
        .categories
        .system
        .roots
        .iter()
        .map(|r| folder_storage(&r.id, &r.display_name, &r.path))
        .collect::<Vec<_>>();
    let system_bytes = system_stores.iter().map(|f| f.bytes).sum::<u64>();
    let system_files = system_stores.iter().map(|f| f.file_count).sum::<u64>();

    let classified = games_bytes
        .saturating_add(artwork_bytes)
        .saturating_add(ai_model_bytes)
        .saturating_add(updates_bytes)
        .saturating_add(logs_bytes)
        .saturating_add(temporary_bytes)
        .saturating_add(system_bytes);
    let other_bytes = used_bytes.saturating_sub(classified);

    let games = category_status(
        games_bytes,
        games_files,
        registry.categories.games.roots.len(),
        used_bytes,
        total_bytes,
        "game files",
        if game_folders.iter().any(|f| f.path_missing()) {
            "warning"
        } else {
            "ok"
        },
        "Per-platform Samba game folders.",
    );
    let artwork = category_status(
        artwork_bytes,
        artwork_files,
        registry.categories.artwork.roots.len(),
        used_bytes,
        total_bytes,
        "artwork files",
        state_for_roots(&artwork_stores),
        "Covers, metadata, generated artwork, and scraper cache.",
    );
    let ai_models_cat = category_status(
        ai_model_bytes,
        ai_model_files.len() as u64,
        registry.categories.ai_models.roots.len(),
        used_bytes,
        total_bytes,
        "model files",
        "ok",
        "Installed Local AI model files and download state.",
    );
    let updates = category_status(
        updates_bytes,
        updates_files,
        registry.categories.updates.roots.len(),
        used_bytes,
        total_bytes,
        "update files",
        state_for_roots(&update_stores),
        "Managed update package/cache roots.",
    );
    let logs = category_status(
        logs_bytes,
        logs_files,
        registry.categories.logs.roots.len(),
        used_bytes,
        total_bytes,
        "log files",
        state_for_roots(&log_stores),
        "Managed log roots only.",
    );
    let temporary = category_status(
        temporary_bytes,
        temporary_files,
        registry.categories.temporary.roots.len(),
        used_bytes,
        total_bytes,
        "temporary files",
        state_for_roots(&temp_stores),
        "Safe temporary roots only.",
    );
    let system = category_status(
        system_bytes,
        system_files,
        registry.categories.system.roots.len(),
        used_bytes,
        total_bytes,
        "system files",
        state_for_roots(&system_stores),
        "Configured system/runtime roots.",
    );
    let other = category_status(
        other_bytes,
        0,
        0,
        used_bytes,
        total_bytes,
        "unclassified",
        "unknown",
        "Used space not classified by the managed storage registry.",
    );

    let ai_rows = ai_model_files
        .iter()
        .map(|m| AiModelDiskStatus {
            friendly_name: m.name.clone(),
            filename: m.filename.clone(),
            size: m.size.clone(),
            status: if m.loaded { "Hot" } else { "Installed" },
        })
        .collect::<Vec<_>>();
    let ai_models = AiModelStorageStatus {
        bytes: ai_model_bytes,
        size: human_size(ai_model_bytes),
        count: ai_model_files.len(),
        meta: format!("{} installed models", ai_model_files.len()),
        detail: if ai_model_files.is_empty() {
            "No local AI model files were found in the Local AI registry roots.".to_string()
        } else {
            "Installed model files are not cache. Remove only unused cold models.".to_string()
        },
        percent_of_total: percent(ai_model_bytes, total_bytes),
        models: ai_rows,
    };

    let diagnostics = storage_diagnostics(
        &registry,
        scan_started
            .elapsed()
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
        &volume,
    );
    let warning_copy = match percent_used {
        98..=100 => "Storage full. Free space before syncing or downloading models.",
        90..=97 => "Storage low. Sync, updates, and model downloads may fail.",
        75..=89 => "Storage is getting full. Review cleanup opportunities before large downloads.",
        _ => "Storage has enough free space for appliance work.",
    };
    StorageStatus {
        scanned_at: now_rfc3339_like(),
        scanning: false,
        scan_error: None,
        total_bytes,
        used_bytes,
        free_bytes,
        percent_used,
        thresholds: StorageThresholds {
            getting_full_percent: 75,
            low_percent: 90,
            full_percent: 98,
        },
        volumes: vec![volume.clone()],
        registry,
        categories: StorageCategories {
            games: games.clone(),
            artwork: artwork.clone(),
            ai_models: ai_models_cat.clone(),
            updates: updates.clone(),
            logs: logs.clone(),
            temporary: temporary.clone(),
            system: system.clone(),
            other: other.clone(),
        },
        game_folders,
        artwork_stores,
        ai_model_files,
        cleanup: CleanupState {
            artwork_bytes_clearable: artwork_bytes,
            temporary_bytes_clearable: temporary_bytes,
            partial_downloads_bytes_clearable: partial_ai_download_roots()
                .iter()
                .map(|p| path_usage(Path::new(p)).bytes)
                .sum(),
            old_update_bytes_clearable: updates_bytes,
            logs_bytes_clearable: logs_bytes,
        },
        diagnostics,
        health,
        header_state,
        header_class,
        header_tooltip: format!(
            "Storage {}: {} free · {}% used",
            health,
            human_size(free_bytes),
            percent_used
        ),
        ok_copy: "Storage has enough free space for appliance work.",
        warning_copy,
        total: human_size(total_bytes),
        used: human_size(used_bytes),
        free: human_size(free_bytes),
        percent: format!("{}%", percent_used),
        games,
        artwork,
        ai_models,
        other,
    }
}

impl GameFolderStorage {
    fn path_missing(&self) -> bool {
        !Path::new(&self.path).exists()
    }
}

fn storage_registry(network: &NetworkStatus) -> StorageRegistry {
    let volume = root_volume();
    let host = hostname();
    let netbios = "HOMECONSOLE";
    let ip = (network.ip_address != "—").then_some(network.ip_address.as_str());
    let game_roots = GAME_SYSTEMS
        .iter()
        .map(|platform| {
            let share = format!("games\\{}", platform);
            let smb_share = format!("games/{}", platform);
            let path = Path::new(GAMES_ROOT)
                .join(platform)
                .to_string_lossy()
                .to_string();
            GameRoot {
                id: (*platform).to_string(),
                platform: (*platform).to_uppercase(),
                display_name: platform_display_name(platform),
                path,
                samba_share_name: format!("games/{}", platform),
                windows_unc: Some(format!(r"\\{}\{}", netbios, share)),
                windows_unc_by_ip: ip.map(|addr| format!(r"\\{}\{}", addr, share)),
                smb_url: Some(format!("smb://{}/{}", host, smb_share)),
                smb_url_by_ip: ip.map(|addr| format!("smb://{}/{}", addr, smb_share)),
            }
        })
        .collect::<Vec<_>>();
    StorageRegistry {
        volumes: vec![volume],
        categories: StorageRegistryCategories {
            games: GameRootsRegistry {
                label: "Games".to_string(),
                roots: game_roots,
            },
            artwork: FolderRootsRegistry {
                label: "Artwork".to_string(),
                roots: vec![
                    folder_root(
                        "artwork-covers",
                        "Covers",
                        &format!("{}/covers", ARTWORK_ROOT),
                        "covers",
                    ),
                    folder_root(
                        "artwork-metadata",
                        "Metadata",
                        &format!("{}/metadata", ARTWORK_ROOT),
                        "metadata",
                    ),
                    folder_root(
                        "artwork-generated",
                        "Generated",
                        &format!("{}/generated", ARTWORK_ROOT),
                        "generated",
                    ),
                    folder_root("artwork-cache", "Scraper Cache", ARTWORK_ROOT, "cache"),
                ],
            },
            ai_models: FolderRootsRegistry {
                label: "AI Models".to_string(),
                roots: model_roots(),
            },
            updates: FolderRootsRegistry {
                label: "Updates".to_string(),
                roots: update_roots()
                    .iter()
                    .enumerate()
                    .map(|(i, p)| {
                        folder_root(&format!("updates-{}", i), "Update Cache", p, "cache")
                    })
                    .collect(),
            },
            logs: FolderRootsRegistry {
                label: "Logs".to_string(),
                roots: log_roots()
                    .iter()
                    .enumerate()
                    .map(|(i, p)| folder_root(&format!("logs-{}", i), "Logs", p, "logs"))
                    .collect(),
            },
            temporary: FolderRootsRegistry {
                label: "Temporary Files".to_string(),
                roots: TEMP_CLEAN_ROOTS
                    .iter()
                    .enumerate()
                    .map(|(i, p)| {
                        folder_root(&format!("temporary-{}", i), "Temporary", p, "temporary")
                    })
                    .collect(),
            },
            system: FolderRootsRegistry {
                label: "System".to_string(),
                roots: vec![
                    folder_root("system-root", "System", "/usr", "system"),
                    folder_root("system-var-lib", "Runtime State", "/var/lib", "system"),
                ],
            },
        },
    }
}

fn folder_root(id: &str, display_name: &str, path: &str, purpose: &str) -> FolderRoot {
    FolderRoot {
        id: id.to_string(),
        display_name: display_name.to_string(),
        path: path.to_string(),
        purpose: Some(purpose.to_string()),
    }
}
fn model_roots() -> Vec<FolderRoot> {
    let mut roots = MODEL_SCAN_ROOTS
        .iter()
        .enumerate()
        .map(|(index, path)| {
            folder_root(
                &format!("ai-installed-models-{}", index),
                "Installed Models",
                path,
                "installed-models",
            )
        })
        .collect::<Vec<_>>();
    roots.extend([
        folder_root(
            "ai-downloads",
            "Downloads",
            "/var/lib/arcadia/model-downloads",
            "downloads",
        ),
        folder_root(
            "ai-partial-downloads",
            "Partial Downloads",
            "/var/lib/arcadia/model-downloads/partial",
            "partial-downloads",
        ),
        folder_root(
            "ai-catalog-cache",
            "Catalog Cache",
            "/var/lib/arcadia/model-catalog",
            "catalog-cache",
        ),
    ]);
    roots
}

fn update_roots() -> Vec<&'static str> {
    vec![
        "/var/cache/pacman/pkg",
        "/var/lib/harmonia/cache",
        "/var/lib/harmonia/artifacts",
    ]
}
fn log_roots() -> Vec<&'static str> {
    vec![
        "/var/log/arcadia",
        "/var/log/homeconsole-sync",
        "/var/lib/harmonia/receipts",
    ]
}
fn partial_ai_download_roots() -> Vec<&'static str> {
    vec![
        "/var/lib/arcadia/model-downloads/partial",
        "/var/cache/arcadia/model-downloads",
    ]
}

fn root_volume() -> StorageVolume {
    let (fs_name, total, used, free, mount) =
        df_row("/").unwrap_or_else(|| (None, 0, 0, 0, "/".to_string()));
    let health = Some(
        match percent(used, total) {
            0..=74 => "ok",
            75..=89 => "warning",
            90..=97 => "warning",
            _ => "error",
        }
        .to_string(),
    );
    StorageVolume {
        id: "root".to_string(),
        label: "Console Storage".to_string(),
        mount_point: mount,
        filesystem: fs_name,
        total_bytes: total,
        used_bytes: used,
        free_bytes: free,
        health,
    }
}

fn df_row(path: &str) -> Option<(Option<String>, u64, u64, u64, String)> {
    let output = Command::new("df").args(["-B1", "-T", path]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.lines().nth(1)?;
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 7 {
        return None;
    }
    Some((
        Some(parts[1].to_string()),
        parts[2].parse().ok()?,
        parts[3].parse().ok()?,
        parts[4].parse().ok()?,
        parts[6].to_string(),
    ))
}

fn game_folder_storage(root: &GameRoot) -> GameFolderStorage {
    let usage = path_usage(Path::new(&root.path));
    let largest_files = largest_files(Path::new(&root.path), 3);
    let synced = load_sync_manifest().map(|entries| {
        entries
            .iter()
            .filter(|e| {
                e.normalized_rom_path
                    .as_ref()
                    .or(e.rom_path.as_ref())
                    .map(|p| p.to_ascii_lowercase().contains(&format!("/{}/", root.id)))
                    .unwrap_or(false)
            })
            .count() as u64
    });
    let unsynced = synced.map(|s| usage.files.saturating_sub(s));
    GameFolderStorage {
        platform: root.platform.clone(),
        display_name: root.display_name.clone(),
        path: root.path.clone(),
        samba_share_name: root.samba_share_name.clone(),
        bytes: usage.bytes,
        size: human_size(usage.bytes),
        file_count: usage.files,
        synced_entries: synced,
        unsynced_files: unsynced,
        largest_files,
        windows_unc: root.windows_unc.clone(),
        windows_unc_by_ip: root.windows_unc_by_ip.clone(),
        smb_url: root.smb_url.clone(),
        smb_url_by_ip: root.smb_url_by_ip.clone(),
    }
}

fn folder_storage(id: &str, display_name: &str, path: &str) -> FolderStorage {
    let usage = path_usage(Path::new(path));
    let state = if !Path::new(path).exists() {
        "unknown"
    } else {
        "ok"
    };
    FolderStorage {
        id: id.to_string(),
        display_name: display_name.to_string(),
        path: path.to_string(),
        bytes: usage.bytes,
        size: human_size(usage.bytes),
        file_count: usage.files,
        last_modified_at: fs::metadata(path)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(system_time_string),
        state: state.to_string(),
    }
}

fn ai_model_files() -> Vec<AIModelStorage> {
    let local_ai = local_ai_status();
    let loaded_name = local_ai.loaded_model.clone();
    let selected_id = local_ai.selected_model_id.clone();
    let mut out = Vec::new();
    for root in model_roots()
        .into_iter()
        .filter(|r| r.purpose.as_deref() == Some("installed-models"))
    {
        let mut found = Vec::new();
        collect_ai_models(Path::new(&root.path), &mut found, 0);
        for (bytes, filename, path) in found {
            let id = model_id(&filename);
            let loaded = loaded_name
                .as_ref()
                .map(|n| n == &filename)
                .unwrap_or(false);
            out.push(AIModelStorage {
                id: id.clone(),
                name: friendly_model_name(&filename),
                filename: filename.clone(),
                path: path.to_string_lossy().to_string(),
                bytes,
                size: human_size(bytes),
                source: "unknown".to_string(),
                loaded,
                selected: selected_id.as_ref().map(|s| s == &id).unwrap_or(false),
                removable: !loaded,
            });
        }
    }
    if let Some(loaded) = loaded_name.as_ref() {
        if !out.iter().any(|model| model.filename == *loaded) {
            let id = model_id(loaded);
            out.push(AIModelStorage {
                id: id.clone(),
                name: friendly_model_name(loaded),
                filename: loaded.clone(),
                path: "Unknown".to_string(),
                bytes: 0,
                size: "Unknown".to_string(),
                source: "unknown".to_string(),
                loaded: true,
                selected: selected_id.as_ref().map(|s| s == &id).unwrap_or(false),
                removable: false,
            });
        }
    }
    out.sort_by(|a, b| b.bytes.cmp(&a.bytes));
    out
}

fn category_status(
    bytes: u64,
    files: u64,
    root_count: usize,
    used: u64,
    total: u64,
    suffix: &str,
    state: &str,
    detail: &str,
) -> StorageCategoryStatus {
    StorageCategoryStatus {
        bytes,
        size: human_size(bytes),
        files,
        file_count: Some(files),
        root_count,
        state: state.to_string(),
        meta: format!("{} {} · {} roots", files, suffix, root_count),
        detail: detail.to_string(),
        percent_of_used: percent(bytes, used),
        percent_of_total: percent(bytes, total),
    }
}
fn state_for_roots(roots: &[FolderStorage]) -> &str {
    if roots.iter().any(|r| r.state == "unknown") {
        "unknown"
    } else {
        "ok"
    }
}

fn largest_files(path: &Path, limit: usize) -> Vec<LargestFile> {
    let mut files = Vec::new();
    collect_largest_files(path, &mut files, 0);
    files.sort_by(|a, b| b.0.cmp(&a.0));
    files
        .into_iter()
        .take(limit)
        .map(|(bytes, path)| LargestFile {
            name: path
                .file_name()
                .and_then(|v| v.to_str())
                .unwrap_or("file")
                .to_string(),
            path: path.to_string_lossy().to_string(),
            bytes,
            size: human_size(bytes),
        })
        .collect()
}
fn collect_largest_files(path: &Path, out: &mut Vec<(u64, PathBuf)>, depth: usize) {
    if depth > 8 {
        return;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        let Ok(m) = e.metadata() else {
            continue;
        };
        if m.is_dir() {
            collect_largest_files(&p, out, depth + 1);
        } else if m.is_file() {
            out.push((m.len(), p));
        }
    }
}

fn storage_diagnostics(
    registry: &StorageRegistry,
    duration: u64,
    volume: &StorageVolume,
) -> StorageDiagnostics {
    let managed = managed_storage_paths_from_registry(registry);
    let missing_dirs = managed
        .iter()
        .filter(|p| !Path::new(p.as_str()).exists())
        .cloned()
        .collect::<Vec<_>>();
    let permission_errors = managed
        .iter()
        .filter(|p| Path::new(p.as_str()).exists() && fs::read_dir(p).is_err())
        .cloned()
        .collect::<Vec<_>>();
    StorageDiagnostics {
        mount_point: volume.mount_point.clone(),
        filesystem: volume.filesystem.clone(),
        scan_duration_ms: duration,
        scanner_version: "arcadia.storage.scan.v1".to_string(),
        missing_dirs,
        permission_errors,
    }
}
fn managed_storage_paths_from_registry(registry: &StorageRegistry) -> Vec<String> {
    let mut paths = Vec::new();
    paths.extend(
        registry
            .categories
            .games
            .roots
            .iter()
            .map(|r| r.path.clone()),
    );
    for cat in [
        &registry.categories.artwork,
        &registry.categories.ai_models,
        &registry.categories.updates,
        &registry.categories.logs,
        &registry.categories.temporary,
        &registry.categories.system,
    ] {
        paths.extend(cat.roots.iter().map(|r| r.path.clone()));
    }
    paths
}
fn is_managed_storage_path(path: &str) -> bool {
    managed_storage_paths_from_registry(&storage_registry(&network_status()))
        .iter()
        .any(|p| p == path)
}

fn cleanup_known_roots(
    action: &'static str,
    roots: Vec<&'static str>,
    success: &str,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    let mut removed = 0u64;
    let mut errors = Vec::new();
    for root in roots {
        match remove_children(Path::new(root)) {
            Ok(count) => removed += count,
            Err(err) => errors.push(format!("{}: {}", root, err)),
        }
    }
    if errors.is_empty() {
        (
            StatusCode::OK,
            Json(ConsoleActionResponse {
                ok: true,
                action,
                command: "arcadia-storage",
                exit_code: Some(0),
                message: format!("{} Removed {} entries.", success, removed),
                stdout: String::new(),
                stderr: String::new(),
            }),
        )
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ConsoleActionResponse {
                ok: false,
                action,
                command: "arcadia-storage",
                exit_code: Some(1),
                message: "Cleanup partially failed.".to_string(),
                stdout: String::new(),
                stderr: errors.join("\n"),
            }),
        )
    }
}

fn platform_display_name(platform: &str) -> String {
    match platform {
        "gba" => "GBA",
        "snes" => "SNES",
        "nes" => "NES",
        "n64" => "N64",
        "ps1" => "PS1",
        "ps2" => "PS2",
        "psp" => "PSP",
        "wii" => "Wii",
        "sega-cd" => "Sega CD",
        "gamecube" => "GameCube",
        "genesis" => "Genesis",
        "dos" => "DOS",
        other => other,
    }
    .to_string()
}
fn now_rfc3339_like() -> String {
    command_stdout("date", &["-Iseconds"]).unwrap_or_else(|| "Unknown".to_string())
}
fn system_time_string(t: SystemTime) -> Option<String> {
    let secs = t.duration_since(UNIX_EPOCH).ok()?.as_secs();
    Some(format!("{}", secs))
}

fn storage_health(percent_used: u8) -> (&'static str, String, &'static str) {
    match percent_used {
        0..=74 => ("OK", "OK".to_string(), "good"),
        75..=89 => ("Getting Full", format!("{}%", percent_used), "warn"),
        90..=97 => ("Low Space", "Low".to_string(), "warn"),
        _ => ("Full", "Full".to_string(), "bad"),
    }
}

#[derive(Default)]
struct Usage {
    bytes: u64,
    files: u64,
}

fn path_usage(path: &Path) -> Usage {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return Usage::default();
    };
    if metadata.file_type().is_symlink() {
        return Usage::default();
    }
    if metadata.is_file() {
        return Usage {
            bytes: metadata.len(),
            files: 1,
        };
    }
    if !metadata.is_dir() {
        return Usage::default();
    }
    let mut usage = Usage::default();
    let Ok(entries) = fs::read_dir(path) else {
        return usage;
    };
    for entry in entries.flatten() {
        let child = path_usage(&entry.path());
        usage.bytes = usage.bytes.saturating_add(child.bytes);
        usage.files = usage.files.saturating_add(child.files);
    }
    usage
}

fn remove_children(path: &Path) -> std::io::Result<u64> {
    if !path.exists() {
        return Ok(0);
    }
    let mut removed = 0u64;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let child = entry.path();
        let metadata = fs::symlink_metadata(&child)?;
        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            fs::remove_dir_all(&child)?;
        } else {
            fs::remove_file(&child)?;
        }
        removed += 1;
    }
    Ok(removed)
}

fn storage_remove_children(
    action: &'static str,
    path: &str,
    success: &str,
    failure: &str,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    match remove_children(Path::new(path)) {
        Ok(count) => (
            StatusCode::OK,
            Json(ConsoleActionResponse {
                ok: true,
                action,
                command: "arcadia-storage",
                exit_code: Some(0),
                message: format!("{} Removed {} entries.", success, count),
                stdout: String::new(),
                stderr: String::new(),
            }),
        ),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ConsoleActionResponse {
                ok: false,
                action,
                command: "arcadia-storage",
                exit_code: Some(1),
                message: failure.to_string(),
                stdout: String::new(),
                stderr: err.to_string(),
            }),
        ),
    }
}

fn collect_ai_models(path: &Path, out: &mut Vec<(u64, String, PathBuf)>, depth: usize) {
    if depth > 8 {
        return;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if metadata.is_dir() {
            collect_ai_models(&path, out, depth + 1);
        } else if metadata.is_file() && is_ai_model_file(&path) {
            let filename = path
                .file_name()
                .and_then(|v| v.to_str())
                .unwrap_or("model")
                .to_string();
            out.push((metadata.len(), filename, path));
        }
    }
}

fn is_ai_model_file(path: &Path) -> bool {
    path.extension()
        .and_then(|v| v.to_str())
        .map(|ext| {
            MODEL_EXTENSIONS
                .iter()
                .any(|candidate| ext.eq_ignore_ascii_case(candidate))
        })
        .unwrap_or(false)
}

fn find_model_path_by_filename(filename: &str) -> Option<PathBuf> {
    let mut models = Vec::new();
    for root in model_roots() {
        collect_ai_models(Path::new(&root.path), &mut models, 0);
    }
    models
        .into_iter()
        .find(|(_, name, _)| name == filename)
        .map(|(_, _, path)| path)
}

fn friendly_model_name(filename: &str) -> String {
    let mut name = filename.to_string();
    for suffix in [".gguf", ".safetensors", ".onnx"] {
        if name.to_lowercase().ends_with(suffix) {
            let new_len = name.len().saturating_sub(suffix.len());
            name.truncate(new_len);
            break;
        }
    }
    name.replace(['_', '-'], " ")
}

fn percent(part: u64, total: u64) -> u8 {
    if total == 0 {
        return 0;
    }
    ((part.saturating_mul(100) / total).min(100)) as u8
}

pub(crate) fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0usize;
    while value >= 1000.0 && unit < UNITS.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} B", bytes)
    } else if value >= 100.0 {
        format!("{:.0} {}", value, UNITS[unit])
    } else {
        format!("{:.1} {}", value, UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_env_uses_screenscraper_api_key_only() {
        let mut lines = Vec::new();
        let mut written = Vec::new();

        push_env_value(
            &mut lines,
            &mut written,
            "SCREENSCRAPER_API_KEY",
            Some("  scrape-secret  ".to_string()),
        );

        assert_eq!(written, vec!["SCREENSCRAPER_API_KEY"]);
        assert_eq!(lines, vec!["SCREENSCRAPER_API_KEY=\"scrape-secret\""]);
        let joined = lines.join("\n");
        assert!(!joined.contains("SCREENSCRAPER_USER"));
        assert!(!joined.contains("SCREENSCRAPER_PASSWORD"));
    }

    #[test]
    fn network_status_parses_ip_addr_global_fallback() {
        let text =
            "2: enp1s0    inet 192.168.123.54/24 brd 192.168.123.255 scope global dynamic enp1s0\n";
        assert_eq!(
            parse_global_ipv4_address(text),
            Some("192.168.123.54".to_string())
        );
    }

    #[test]
    fn provider_keys_ui_exposes_api_key_not_username_password() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.home.arpa/".to_string(),
            product: "HomeConsole".to_string(),
        };
        let status = console_status(&state);
        let rendered = ui::layout(&status).into_string();

        assert!(rendered.contains("ScreenScraper API key"));
        assert!(rendered.contains("screenscraper_api_key"));
        assert!(!rendered.contains("screenscraper_user"));
        assert!(!rendered.contains("screenscraper_password"));
        assert!(!rendered.contains("ScreenScraper user"));
    }

    #[test]
    fn provider_keys_script_posts_screenscraper_api_key_only() {
        assert!(APP_JS.contains("screenscraper_api_key"));
        assert!(!APP_JS.contains("screenscraper_user"));
        assert!(!APP_JS.contains("screenscraper_password"));
    }

    #[test]
    fn toasts_are_clickable_dismiss_controls() {
        assert!(APP_JS.contains("document.createElement('button')"));
        assert!(APP_JS.contains("dismiss notification"));
        assert!(APP_JS.contains("node.addEventListener('click', () => node.remove())"));
        assert!(APP_JS.contains("toast-dismiss"));
        assert!(APP_CSS.contains(".toast:focus-visible"));
        assert!(APP_CSS.contains("cursor: pointer"));
    }

    #[test]
    fn human_text_font_sizes_stay_inside_ordinary_bounds() {
        let allowed_large_icon_selectors = [".product-mark", ".launcher-icon", ".home-action-icon"];

        for (index, line) in APP_CSS.lines().enumerate() {
            if !line.contains("font-size:") {
                continue;
            }
            if allowed_large_icon_selectors
                .iter()
                .any(|selector| line.contains(selector))
            {
                continue;
            }

            let font_size = line.split("font-size:").nth(1).unwrap_or_default();
            for part in font_size.split("px") {
                let value = part
                    .rsplit(|c: char| !(c.is_ascii_digit() || c == '.'))
                    .next()
                    .unwrap_or_default();
                if value.is_empty() {
                    continue;
                }
                let parsed: f32 = value.parse().expect("font-size px value parses");
                assert!(
                    parsed <= 22.0,
                    "human text font-size above 22px on CSS line {}: {}",
                    index + 1,
                    line
                );
            }
        }
    }

    #[test]
    fn home_view_is_operational_surface_without_duplicate_navigation() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.home.arpa/".to_string(),
            product: "HomeConsole".to_string(),
        };
        let status = console_status(&state);
        let rendered = ui::layout(&status).into_string();
        let home_start = rendered
            .find("<section id=\"view-home\"")
            .expect("home view starts");
        let home_end = home_start
            + rendered[home_start..]
                .find("<section id=\"view-games\"")
                .expect("games view follows home");
        let home_html = &rendered[home_start..home_end];

        for required in [
            "priority-strip",
            "home-operational-grid",
            "Storage",
            "storage-bar",
            "Games",
            "Artwork",
            "AI Models",
            "Other",
            "Console",
            "Local AI",
            "Model",
            "GPU",
            "LAN",
            "Game Library",
            "Detected",
            "Synced",
            "Last sync",
        ] {
            assert!(home_html.contains(required), "missing {required}");
        }

        for forbidden in [
            "Console Home",
            "HomeConsole Launchpad",
            "What do you want to do?",
            "Add Games",
            "Console Status",
            "Recent Activity",
            "home-action-tile",
        ] {
            assert!(
                !home_html.contains(forbidden),
                "duplicated nav/meta survived: {forbidden}"
            );
        }

        assert!(
            !home_html.contains(">Home<"),
            "home title leaked into viewport"
        );
        assert!(APP_CSS.contains(".priority-strip"));
        assert!(APP_CSS.contains(".home-operational-grid"));
        assert!(!home_html.contains("Now"));
        assert!(!home_html.contains("Games ready to sync"));
    }

    #[test]
    fn local_ai_language_replaces_ai_model_jargon() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.home.arpa/".to_string(),
            product: "HomeConsole".to_string(),
        };
        let status = console_status(&state);
        let rendered = ui::layout(&status).into_string();

        for required in [
            "Local AI",
            "Runtime",
            "Loaded Model",
            "Installed Models",
            "Get Models",
            "Inference",
            "GPU &amp; Storage",
            "Activity",
            "Install Inharmonia",
            "Hugging Face model",
            "Copy Endpoint",
        ] {
            assert!(rendered.contains(required), "missing {required}");
        }

        for forbidden in ["Load AI Model", "Model Manager", "LLM"] {
            assert!(
                !rendered.contains(forbidden),
                "forbidden visible term survived: {forbidden}"
            );
        }
    }

    #[test]
    fn sync_view_is_ceremonial_workflow_not_log_first() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.home.arpa/".to_string(),
            product: "HomeConsole".to_string(),
        };
        let status = console_status(&state);
        let rendered = ui::layout(&status).into_string();
        let sync_start = rendered
            .find("<section id=\"view-sync\"")
            .expect("sync view starts");
        let sync_end = sync_start
            + rendered[sync_start..]
                .find("<section id=\"view-storage\"")
                .expect("storage follows sync");
        let sync_html = &rendered[sync_start..sync_end];

        for required in [
            "I copy games into folders. Sync turns those files into a usable console library.",
            "Start Sync",
            "data-action=\"sync-games\"",
            "Copy Games",
            "Copy game files into the matching console folders over the network.",
            "Scan Library",
            "The console scans game folders and detects new, changed, or removed files.",
            "Fetch Artwork",
            "Optional metadata providers improve titles, covers, and artwork.",
            "Create Game Entries",
            "The console creates or updates GameScope library entries for detected games.",
            "Available in GameScope",
            "Synced games appear in the GameScope library after sync completes.",
            "Waiting",
            "Running",
            "Complete",
            "Skipped",
            "Error",
            "Games found",
            "New games",
            "Removed games",
            "Changed files",
            "Artwork found",
            "Artwork missing",
            "Provider key status",
            "Configure Metadata Providers",
            "Games still work without artwork keys.",
            "Entries created",
            "Entries updated",
            "Entries skipped",
            "Last successful sync",
            "Total synced games",
            "GameScope state",
            "GameScope is running. Synced games should appear after sync completes.",
            "Result Summary",
            "New entries created",
            "Artwork downloaded",
            "Duration",
            "Completed time",
            "Output",
            "Metadata keys are optional. They improve artwork and titles, but games can still sync without them.",
            "SteamGridDB",
            "TheGamesDB",
            "ScreenScraper",
        ] {
            assert!(sync_html.contains(required), "missing {required}");
        }

        let workflow = sync_html.find("sync-workflow").expect("workflow shown");
        let output = sync_html
            .find("sync-output-panel")
            .expect("output available");
        assert!(
            workflow < output,
            "workflow appears before the collapsed output"
        );
        for forbidden in ["View Sync Log", "Logs are secondary"] {
            assert!(
                !sync_html.contains(forbidden),
                "rejected sync copy survived: {forbidden}"
            );
        }
        assert!(sync_html.contains("data-storage-health=\"OK\""));
        assert!(sync_html.contains("data-sync-step=\"1\""));
        assert!(sync_html.contains("data-sync-step=\"5\""));
        for forbidden in ["ROM parser", "shortcut VDF", "SteamGrid pipeline"] {
            assert!(
                !sync_html.contains(forbidden),
                "developer jargon leaked: {forbidden}"
            );
        }
        assert!(APP_JS.contains("Scanning game folders…"));
        assert!(APP_JS.contains("Fetching artwork…"));
        assert!(APP_JS.contains("Creating GameScope entries…"));
        assert!(APP_JS.contains("Sync complete. Your games are ready in GameScope."));
        assert!(APP_JS.contains("Storage is full. Free space before syncing games."));
    }

    #[test]
    fn storage_view_answers_where_disk_space_went_safely() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.home.arpa/".to_string(),
            product: "HomeConsole".to_string(),
        };
        let status = console_status(&state);
        let rendered = ui::layout(&status).into_string();

        for required in [
            "data-view=\"storage\"",
            "view-storage",
            "Storage",
            "Storage OK",
            "free",
            "used",
            "Games",
            "Artwork &amp; Metadata",
            "AI Models",
            "Updates",
            "Logs",
            "Temp",
            "System",
            "Games by Folder",
            "Copy path",
            "Clear Artwork Cache",
            "Clear Partial Downloads",
            "Locations",
            "Diagnostics",
            "data-nav-target=\"storage\"",
        ] {
            assert!(rendered.contains(required), "missing {required}");
        }
        assert!(
            rendered.contains("Remove Model") || rendered.contains("No models installed"),
            "storage page must either show removable models or the true empty model state"
        );
        assert!(rendered.contains("/api/storage/cleanup/artwork"));
        assert!(rendered.contains("/api/storage/cleanup/temporary"));
        assert!(rendered.contains("/api/storage/rescan"));
        assert!(rendered.contains("data-storage-modal=\"games\""));
        assert!(rendered.contains("data-storage-modal-template=\"games\""));
        assert!(!rendered.contains("<details class=\"storage-section"));
        assert!(!rendered.contains("delete-all-games"));
    }

    #[test]
    fn network_view_and_home_contract_follow_sidebar_boundary() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://arcadia.home.arpa/".to_string(),
            product: "HomeConsole".to_string(),
        };
        let status = console_status(&state);
        let rendered = ui::layout(&status).into_string();

        let home_start = rendered
            .find("<section id=\"view-home\"")
            .expect("home view starts");
        let home_end = home_start
            + rendered[home_start..]
                .find("<section id=\"view-games\"")
                .expect("games follows home");
        let home_html = &rendered[home_start..home_end];
        for required in [
            "priority-strip",
            "Storage",
            "Game Library",
            "Local AI",
            "storage-bar",
        ] {
            assert!(home_html.contains(required), "missing home {required}");
        }
        for forbidden in [
            "Console Home",
            "HomeConsole Launchpad",
            "What do you want to do?",
            "home-action-tile",
            "Add Games",
            "Console Status",
            "Recent Activity",
        ] {
            assert!(
                !home_html.contains(forbidden),
                "home duplicated navigation/meta: {forbidden}"
            );
        }

        assert!(rendered.contains("data-view=\"network\""));
        let network_start = rendered
            .find("id=\"view-network\"")
            .expect("network view starts");
        let network_end = network_start
            + rendered[network_start..]
                .find("<section id=\"view-access-pin\"")
                .expect("access follows network");
        let network_html = &rendered[network_start..network_end];
        for required in [
            "Online",
            "Active connection",
            "IP address",
            "Gateway",
            "DNS",
            "LAN",
            "Internet",
            "Wi-Fi",
            "Wired LAN",
            "Details",
            "IP Settings",
            "Services",
            "Diagnostics",
            "http://arcadia.home.arpa",
        ] {
            assert!(
                network_html.contains(required),
                "missing network {required}"
            );
        }
        for forbidden in ["nmcli", "iwctl", "ip addr", "saved password"] {
            assert!(
                !network_html.contains(forbidden),
                "network leaked raw/secret term: {forbidden}"
            );
        }
        assert!(!network_html.contains("wifi-network-list"));
        assert!(!network_html.contains("data-network-connect-form"));
        assert!(!network_html.contains("Advanced IP Settings"));
        assert!(APP_JS.contains("function openWifiNetworkPicker"));
        assert!(APP_JS.contains("function normalizeWifiNetworks"));
        assert!(APP_JS.contains("input.type = event.target.checked ? 'text' : 'password'"));
        assert!(!home_html.contains("Now"));
        assert!(!home_html.contains("Games ready to sync"));
    }

    #[test]
    fn network_state_payload_matches_appliance_contract() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.home.arpa/".to_string(),
            product: "HomeConsole".to_string(),
        };
        let payload = network_state(&state);
        let json = serde_json::to_string(&payload).expect("network state serializes");
        for required in [
            "appliance",
            "activeConnection",
            "ethernet",
            "wifi",
            "services",
            "webConsole",
            "lanInference",
            "savedNetworks",
            "scanResults",
        ] {
            assert!(
                json.contains(required),
                "missing network payload field {required}"
            );
        }
        assert!(!json.contains("smb:://"));
        assert!(!json.contains("smb:/homeconsole"));
        assert!(!json.contains("\\undefined"));
        assert!(!json.contains("smb://undefined"));
    }

    #[test]
    fn system_view_replaces_advanced_with_structured_support_panel() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.home.arpa/".to_string(),
            product: "HomeConsole".to_string(),
        };
        let status = console_status(&state);
        let rendered = ui::layout(&status).into_string();
        let system_start = rendered
            .find("id=\"view-system\"")
            .expect("system view starts");
        let system_html = &rendered[system_start..];

        assert!(rendered.contains("data-view=\"system\""));
        assert!(!rendered.contains("data-view=\"power\""));
        assert!(!rendered.contains("id=\"view-power\""));
        for required in [
            "System",
            "Power",
            "Console controls",
            "Full system reboot",
            "Power off appliance",
            "Game session only",
            "reboot-console",
            "shutdown-console",
            "SSH status",
            "Disabled",
            "Hostname",
            "LAN IP address",
            "Username",
            "ssh console@console.home.arpa",
            "Enable SSH",
            "Disable SSH",
            "Copy SSH Command",
            "GameScope",
            "Samba",
            "Game Sync",
            "Local AI",
            "Local AI Inference",
            "Web GUI",
            "Restart",
            "Sync",
            "View",
            "Copy",
            "Download",
            "Local domain/path",
            "MAC address",
            "Status",
            "Active interface",
            "Web GUI",
            "http://console.home.arpa",
            "Games Folder",
            "\\\\HOMECONSOLE",
            "http://console.home.arpa:7777",
            "80/443",
            "445",
            "7777",
            "22",
        ] {
            assert!(system_html.contains(required), "missing {required}");
        }

        let first_log_group = system_html.find(">Sync<").expect("sync group shown");
        let service_row = system_html.find("GameScope").expect("service row shown");
        assert!(
            service_row < first_log_group,
            "service rows precede event groups"
        );
        assert!(rendered.contains("data-nav-target=\"system\""));
        assert!(APP_JS.contains("if (view === 'advanced') view = 'system';"));
        assert!(APP_JS.contains("Restarting GameScope may close the active game session."));
        assert!(!rendered.contains("data-view=\"advanced\""));
        assert!(!rendered.contains(">Advanced<"));
        let forbidden = [
            "Expert Mode".to_string(),
            "Developer".to_string(),
            ["View", "Logs"].join(" "),
            ["Logs help", "diagnose problems"].join(" "),
            ["View technical", "console status"].join(" "),
            ["view", "heading"].join("-"),
            "Networking".to_string(),
            "Health for the console services".to_string(),
            "How the console is reached".to_string(),
            "Open local ports".to_string(),
            "Network status".to_string(),
            "Sync Log".to_string(),
            "Local AI Log".to_string(),
            "System Log".to_string(),
            "SSH is for direct technical access".to_string(),
            "Normal game management does not require SSH".to_string(),
            "Only enable SSH on a trusted home network".to_string(),
            "LAN Inference is intended only for trusted home networks".to_string(),
            "Runs the console gaming session".to_string(),
            "Shares game folders".to_string(),
            "Adds copied games".to_string(),
            "Loads the selected local AI model".to_string(),
            "Lets other home-network devices".to_string(),
            "Runs this management interface".to_string(),
        ];
        for forbidden in forbidden {
            assert!(
                !system_html.contains(&forbidden),
                "forbidden System label survived: {forbidden}"
            );
        }
    }

    #[test]
    fn local_ai_state_payload_matches_manager_contract() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.home.arpa/".to_string(),
            product: "HomeConsole".to_string(),
        };
        let payload = local_ai_state(&state);
        let json = serde_json::to_string(&payload).expect("local ai state serializes");
        for required in [
            "runtime",
            "loadedModel",
            "installedModels",
            "recommendedModels",
            "downloads",
            "inference",
            "hardware",
            "Inharmonia",
        ] {
            assert!(json.contains(required), "missing local ai field {required}");
        }
        assert!(!json.to_ascii_lowercase().contains("password"));
        assert!(!json.to_ascii_lowercase().contains("api_key"));
    }

    #[test]
    fn appliance_shell_renders_required_viewports_and_no_vault_indicator() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.home.arpa/".to_string(),
            product: "HomeConsole".to_string(),
        };
        let status = console_status(&state);
        let rendered = ui::layout(&status).into_string();

        for view in [
            "view-home",
            "view-games",
            "view-sync",
            "view-storage",
            "view-local-ai",
            "view-network",
            "view-access-pin",
            "view-updates",
            "view-system",
        ] {
            assert!(rendered.contains(view), "missing {view}");
        }
        for indicator in [
            "GameScope",
            "Storage",
            "Sync",
            "Local AI",
            "Updates",
            "Access / PIN",
        ] {
            assert!(rendered.contains(indicator), "missing {indicator}");
        }
        let header_start = rendered
            .find("<header class=\"top-header\"")
            .expect("top header rendered");
        let header_end = rendered
            .find("<div class=\"workspace\"")
            .expect("workspace follows header");
        let header_html = &rendered[header_start..header_end];
        for required in [
            "HomeConsole",
            "Network",
            "Games",
            "Updates",
            "Uptime",
            "AI",
            "Lock",
            "data-chip-kind=\"network\"",
            "data-chip-kind=\"games\"",
            "data-chip-kind=\"updates\"",
            "data-chip-kind=\"uptime\"",
            "data-chip-kind=\"local-ai\"",
            "data-chip-kind=\"pin\"",
        ] {
            assert!(
                header_html.contains(required),
                "missing header currentness item: {required}"
            );
        }
        for forbidden in [
            "Arcadia Console",
            "GameScope",
            "Storage",
            "UI contract",
            "Vault status",
        ] {
            assert!(
                !header_html.contains(forbidden),
                "developer/proof header item survived: {forbidden}"
            );
        }
        assert!(!rendered.contains("smb:://"));
        assert!(!rendered.contains("Vault"));
        assert!(rendered.contains("\\\\HOMECONSOLE"));
        assert!(rendered.contains("http://console.home.arpa:7777"));
        assert!(!rendered.contains(r#"data-view="lan-inference""#));
        assert!(!rendered.contains("view-lan-inference"));
        assert!(APP_JS.contains("view === 'ai-model' || view === 'lan-inference'"));
    }

    #[test]
    fn appliance_css_uses_desktop_density_not_jumbo_display_type() {
        for forbidden in [
            "font-size: clamp(34px",
            "font-size: clamp(24px",
            "font-size: 64px",
            "font-size: 38px",
            "font-size: 36px",
            "font-size: 34px",
            "font-size: 32px",
            "font-size: 28px",
            "font-size: 26px",
        ] {
            assert!(
                !APP_CSS.contains(forbidden),
                "oversized CSS survived: {forbidden}"
            );
        }
        let forbidden = [
            [".view", "heading"].join("-"),
            ["min-height:", "230px"].join(" "),
            ["min-height:", "66px"].join(" "),
            ["font-size:", "28px"].join(" "),
            ["font-size:", "26px"].join(" "),
        ];
        for forbidden in forbidden {
            assert!(
                !APP_CSS.contains(&forbidden),
                "oversized/header CSS survived: {forbidden}"
            );
        }
        assert!(APP_CSS.contains(".path-card code { display: block; margin-top: 7px; color: var(--orange-strong); font-size: 20px;"));
        assert!(APP_CSS.contains(".status-card strong, .active-model strong { display: block; margin: 5px 0 7px; font-size: 18px;"));
    }
}

mod anyhow_free {
    pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
}
