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
struct ProviderKeysRequest {
    steamgriddb_api_key: Option<String>,
    thegamesdb_api_key: Option<String>,
    screenscraper_api_key: Option<String>,
}

#[derive(Serialize)]
struct ProviderKeyStatus {
    id: &'static str,
    env_key: &'static str,
    configured: bool,
}

#[derive(Serialize)]
struct ProviderKeysStatusResponse {
    ok: bool,
    action: &'static str,
    path: &'static str,
    providers: Vec<ProviderKeyStatus>,
    message: String,
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

#[derive(Deserialize)]
struct SshKeyInstallRequest {
    public_key: String,
}

#[derive(Deserialize)]
struct TrustModeRequest {
    mode: String,
    confirm: Option<String>,
}

#[derive(Deserialize)]
struct RootCaInstallRequest {
    ca_bundle: String,
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

#[derive(Deserialize)]
struct WifiConnectRequest {
    ssid: String,
    password: Option<String>,
}
#[derive(Deserialize)]
struct WifiForgetRequest {
    ssid: String,
}

#[derive(Deserialize)]
struct HarmoniaModuleToggleRequest {
    module_id: String,
    enabled: bool,
}

#[derive(Deserialize)]
struct HarmoniaLedgerQuery {
    page: Option<usize>,
    per_page: Option<usize>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HarmoniaLedgerEntry {
    ordinal: usize,
    stamp: String,
    schema: String,
    profile_id: String,
    module_id: String,
    ok: Option<bool>,
    changed: Option<bool>,
    first_missing_signal: String,
    receipt_dir: String,
    entry: serde_json::Value,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HarmoniaLedgerResponse {
    ok: bool,
    action: &'static str,
    profile_id: &'static str,
    ledger_path: &'static str,
    page: usize,
    per_page: usize,
    total_entries: usize,
    total_pages: usize,
    entries: Vec<HarmoniaLedgerEntry>,
    message: String,
}
