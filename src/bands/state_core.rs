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
    #[serde(skip_serializing_if = "Option::is_none")]
    build_sha: Option<&'static str>,
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
    pub vault: VaultStatus,
    pub identity: IdentityStatus,
    pub surfaces: SurfaceStatus,
    pub samba: SambaStatus,
    pub storage: StorageStatus,
    pub network: NetworkStatus,
    pub library: LibraryStatus,
    pub local_ai: LocalAiStatus,
    pub controllers: ControllerStatus,
    pub updates: UpdatesStatus,
    pub system: SystemAdminStatus,
    pub benchmark: BenchmarkAggregate,
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
#[serde(rename_all = "camelCase")]
pub struct SystemAdminStatus {
    pub ssh: SshAccessStatus,
    pub trust: TrustStatus,
    pub services: Vec<SystemServiceStatus>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SshAccessStatus {
    pub service_state: String,
    pub password_auth: String,
    pub hostname: String,
    pub username: String,
    pub lan_ip: String,
    pub command: String,
    pub authorized_keys_path: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustStatus {
    pub mode: String,
    pub ca_installed: bool,
    pub ca_subject: Option<String>,
    pub ca_issuer: Option<String>,
    pub ca_not_after: Option<String>,
    pub ca_path: &'static str,
    pub https_probe_url: &'static str,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemServiceStatus {
    pub name: String,
    pub state: String,
    pub detail: String,
    pub action: Option<String>,
    pub endpoint: Option<String>,
}

#[derive(Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ControllerTuningStatus {
    pub left_stick_deadzone: f32,
    pub right_stick_deadzone: f32,
    pub left_stick_sensitivity: f32,
    pub right_stick_sensitivity: f32,
}

impl ControllerTuningStatus {
    pub fn defaults() -> Self {
        Self {
            left_stick_deadzone: 0.15,
            right_stick_deadzone: 0.15,
            left_stick_sensitivity: 1.0,
            right_stick_sensitivity: 1.0,
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ControllerPoolEntry {
    pub id: String,
    pub name: String,
    pub handler: String,
    pub path: String,
    pub glyph: String,
    pub transport: String,
    pub kind: String,
    pub state: String,
    pub layout_style: String,
    pub tuple_count: usize,
    pub bindings: Vec<ControllerBindingStatus>,
    pub tuning: ControllerTuningStatus,
    pub last_seen: String,
    pub selected: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ControllerStatus {
    pub state: String,
    pub detected_count: usize,
    pub primary_device: String,
    pub active_controller_id: String,
    pub last_scan: String,
    pub recovery: ControllerRecoveryStatus,
    pub devices: Vec<ControllerDeviceStatus>,
    pub controller_pool: Vec<ControllerPoolEntry>,
    pub profile: ControllerProfileStatus,
    pub profile_presets: Vec<ControllerProfilePresetStatus>,
    pub live_input: ControllerInputStatus,
    pub emulators: Vec<EmulatorControllerStatus>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ControllerDeviceStatus {
    pub name: String,
    pub handler: String,
    pub kind: String,
    pub path: String,
    pub state: String,
    pub transport: String,
    pub glyph: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ControllerRecoveryStatus {
    pub state: String,
    pub title: String,
    pub detail: String,
    pub action: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ControllerProfilePresetStatus {
    pub name: String,
    pub layout: String,
    pub description: String,
    pub state: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ControllerBindingStatus {
    pub control: String,
    pub binding: String,
    pub pressed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub axis_value: Option<i16>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ControllerInputStatus {
    pub state: String,
    pub device: String,
    pub sample_path: String,
    pub pressed: Vec<ControllerBindingStatus>,
    pub axes: Vec<ControllerBindingStatus>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ControllerProfileStatus {
    pub state: String,
    pub name: String,
    pub path: String,
    pub bindings: Vec<ControllerBindingStatus>,
    pub tuning: ControllerTuningStatus,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmulatorControllerStatus {
    pub emulator: String,
    pub command: String,
    pub state: String,
    pub config_path: String,
    pub mapping_path: String,
    pub profile: String,
}

#[derive(Clone, Serialize)]
pub struct UiContract {
    pub schema: &'static str,
    pub button_variants: [&'static str; 3],
    pub composition: &'static str,
    pub modal: &'static str,
}

#[derive(Clone, Serialize)]
pub struct VaultStatus {
    pub present: bool,
    pub mounted: bool,
    pub auto_decrypt_enabled: bool,
    pub unlock_required: bool,
}

#[derive(Clone, Serialize)]
pub struct GuiPinStatus {
    pub pin_required: bool,
    pub authority: &'static str,
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
    pub warnings: Vec<String>,
    pub overlap_warnings: Vec<String>,
    pub category_scan_errors: Vec<String>,
    pub last_scan_timestamp: String,
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
