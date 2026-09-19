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
    pub profile_id: String,
    pub identity: String,
    pub suite_ok: bool,
    pub suite_changed: bool,
    pub check_ok: bool,
    pub check_changed: bool,
    pub check_missing_signal: String,
    pub first_missing_signal: String,
    pub module_count: usize,
    pub operation_count: usize,
    pub last_update_run: String,
    pub pending_updates: usize,
    pub latest_receipt: String,
    pub latest_check_receipt: String,
    pub module_root: String,
    pub modules: Vec<HarmoniaModuleStatus>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HarmoniaModuleStatus {
    pub id: String,
    pub label: String,
    pub description: String,
    pub enabled: bool,
    pub present: bool,
    pub state: String,
    pub receipt_path: String,
    pub pinned_module_membership: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HarmoniaModuleToggleResponse {
    ok: bool,
    action: &'static str,
    module_id: String,
    enabled: bool,
    profile_path: &'static str,
    message: String,
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
    pub timezone: Option<String>,
    pub ntp_synchronized: Option<bool>,
    pub connection_session_detail: String,
    pub resolv_nameservers: String,
    pub resolv_search: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SpeedTestResponse {
    ok: bool,
    action: &'static str,
    download_mbps: Option<f64>,
    duration_ms: Option<u64>,
    bytes: Option<u64>,
    message: String,
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
