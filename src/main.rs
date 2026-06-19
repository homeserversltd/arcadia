use axum::{
    extract::State,
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
    env, fs,
    fs::OpenOptions,
    io::Write,
    net::SocketAddr,
    path::Path,
    process::{Command, Stdio},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
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
    pub surfaces: SurfaceStatus,
    pub ui_contract: UiContract,
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
pub struct SurfaceStatus {
    pub http: &'static str,
    pub mdns: &'static str,
    pub smb: &'static str,
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
        .route("/api/gui-pin/status", get(gui_pin_status_route))
        .route("/api/gui-pin/access", post(set_gui_pin_access))
        .route("/api/gui-pin/change", post(change_gui_pin))
        .route("/api/provider-keys/save", post(save_provider_keys))
        .route("/api/actions/update-gui", post(action_update_gui))
        .route("/api/actions/sync-games", post(action_sync_games))
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

async fn gui_pin_status_route() -> Json<GuiPinStatus> {
    Json(gui_pin_status())
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
    ConsoleStatus {
        schema: "arcadia.status.v6",
        product: state.product.clone(),
        canonical_url: state.canonical_url.clone(),
        arcadia: ArcadiaStatus {
            service: "arcadia",
            version: env!("CARGO_PKG_VERSION"),
            mode: "unity-appliance-shell",
            ui: "top-header-left-launcher-focused-viewports",
        },
        runtime: runtime_status(state.started_unix),
        gui_pin: gui_pin_status(),
        surfaces: SurfaceStatus {
            http: "console.home.arpa -> :8080",
            mdns: "homeconsole.local",
            smb: "HOMECONSOLE",
        },
        ui_contract: UiContract {
            schema: "arcadia.ui.contract.v5",
            button_variants: ["primary", "secondary", "danger"],
            composition: "top header status badges, Ubuntu-style left launcher, one focused viewport at a time",
            modal: "confirmation/readback only; GUI PIN changes post to local root-owned helpers",
        }
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
    fn home_view_is_action_launchpad_before_status() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.home.arpa/".to_string(),
            product: "HomeConsole".to_string(),
        };
        let status = console_status(&state);
        let rendered = ui::layout(&status).into_string();

        let title = rendered.find("Console Home").expect("home title rendered");
        let action = rendered
            .find("What do you want to do?")
            .expect("primary action section rendered");
        let status_title = rendered
            .find("Console Status")
            .expect("status section still rendered");
        assert!(title < action, "intro appears before action launchpad");
        assert!(
            action < status_title,
            "action launchpad appears before status cards"
        );

        for required in [
            "Add Games",
            "Open the console’s network folders and copy games into the right system folder.",
            "data-nav-target=\"games\"",
            "Sync Games",
            "Scan the game folders, fetch artwork, and add games to the GameScope library.",
            "data-nav-target=\"sync\"",
            "Local AI",
            "Choose the local AI that runs on this console and can be used on your home network.",
            "data-nav-target=\"ai-model\"",
            "home-action-tile",
        ] {
            assert!(rendered.contains(required), "missing {required}");
        }

        for forbidden_on_home in ["SSH", "Open ports"] {
            let home_end = rendered.find("id=\"view-games\"").unwrap_or(rendered.len());
            assert!(
                !rendered[..home_end].contains(forbidden_on_home),
                "advanced label leaked into Home: {forbidden_on_home}"
            );
        }
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
            "Choose the local AI that runs on this console and can be used on your home network.",
            "Loaded Model",
            "Available Models",
            "GPU Usage",
            "Load This Model",
            "Not Loaded",
            "Open LAN Inference Settings",
            "Mistral 7B Instruct",
            "mistral-7b-instruct.Q4_K_M.gguf",
            "Recommended use",
        ] {
            assert!(rendered.contains(required), "missing {required}");
        }

        for forbidden in ["Load AI Model", "Model Manager", "LLM", "llama.cpp"] {
            assert!(
                !rendered.contains(forbidden),
                "forbidden visible term survived: {forbidden}"
            );
        }
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
            "See what is using space on the console.",
            "Free Space",
            "Total storage",
            "Used storage",
            "Free storage",
            "Percent used",
            "Storage OK",
            "Games",
            "Artwork",
            "AI Models",
            "Other Storage",
            "Open Games Folder",
            "Clear Artwork Cache",
            "Clearing artwork does not delete games. Artwork can be downloaded again during Sync.",
            "Rebuild Artwork on Next Sync",
            "Remove Model",
            "This removes the model file from console storage. It does not affect games.",
            "Clean Temporary Files",
            "Storage is low. Sync may fail if artwork or shortcuts cannot be written.",
            "Storage is low. Remove unused games, artwork, or AI models before adding more models.",
            "data-nav-target=\"storage\"",
        ] {
            assert!(rendered.contains(required), "missing {required}");
        }

        let storage_start = rendered
            .find("id=\"view-storage\"")
            .expect("storage view starts");
        let storage_end = rendered
            .find("id=\"view-ai-model\"")
            .expect("local ai follows storage");
        let storage_html = &rendered[storage_start..storage_end];
        for forbidden in ["/home", "/var", "/mnt", "/opt", "delete-all-games"] {
            assert!(
                !storage_html.contains(forbidden),
                "raw/dangerous storage term leaked: {forbidden}"
            );
        }
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
            "view-ai-model",
            "view-lan-inference",
            "view-access-pin",
            "view-updates",
            "view-power",
            "view-advanced",
        ] {
            assert!(rendered.contains(view), "missing {view}");
        }
        for indicator in [
            "Network",
            "GameScope",
            "Storage",
            "Sync",
            "AI",
            "Update",
            "PIN",
        ] {
            assert!(rendered.contains(indicator), "missing {indicator}");
        }
        assert!(!rendered.contains("Vault"));
        assert!(rendered.contains("\\\\HOMECONSOLE"));
        assert!(rendered.contains("http://console.home.arpa:7777"));
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
        assert!(APP_CSS.contains(".view-heading h2 { margin: 0; font-size: 22px;"));
        assert!(APP_CSS.contains(".path-card code { display: block; margin-top: 7px; color: var(--orange-strong); font-size: 20px;"));
        assert!(APP_CSS.contains(".status-card strong, .active-model strong { display: block; margin: 5px 0 7px; font-size: 18px;"));
    }
}

mod anyhow_free {
    pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
}
