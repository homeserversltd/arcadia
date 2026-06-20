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

