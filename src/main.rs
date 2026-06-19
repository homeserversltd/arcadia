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
    net::SocketAddr,
    path::{Path, PathBuf},
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
const GAMES_ROOT: &str = "/home/owner/Games";
const ARTWORK_ROOT: &str = "/home/owner/Games/artwork";
const TEMP_CLEAN_ROOTS: [&str; 2] = ["/tmp", "/var/tmp"];
const MODEL_SCAN_ROOTS: [&str; 3] = ["/home/owner", "/opt", "/var/lib"];
const MODEL_EXTENSIONS: [&str; 3] = ["gguf", "safetensors", "onnx"];
const GAME_SYSTEMS: [&str; 12] = [
    "gba", "genesis", "snes", "nes", "ps1", "n64", "ps2", "sega-cd", "psp", "gamecube", "wii",
    "dos",
];

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
    pub storage: StorageStatus,
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
pub struct StorageStatus {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
    pub percent_used: u8,
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
pub struct StorageCategoryStatus {
    pub bytes: u64,
    pub size: String,
    pub files: u64,
    pub meta: String,
    pub detail: String,
    pub percent_of_total: u8,
}

#[derive(Clone, Serialize)]
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
pub struct AiModelDiskStatus {
    pub friendly_name: String,
    pub filename: String,
    pub size: String,
    pub status: &'static str,
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
        storage: storage_status(),
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

fn storage_status() -> StorageStatus {
    let disk = root_disk_usage().unwrap_or((0, 0, 0));
    let (total_bytes, used_bytes, free_bytes) = disk;
    let percent_used = percent(used_bytes, total_bytes);
    let (health, header_state, header_class) = storage_health(percent_used);
    let games = game_storage(total_bytes);
    let artwork = category_from_path(
        Path::new(ARTWORK_ROOT),
        total_bytes,
        "artwork files",
        "Last artwork sync: Not reported.",
    );
    let ai_models = ai_model_storage(total_bytes);
    let classified = games
        .bytes
        .saturating_add(artwork.bytes)
        .saturating_add(ai_models.bytes);
    let other_bytes = used_bytes.saturating_sub(classified);
    let other = StorageCategoryStatus {
        bytes: other_bytes,
        size: human_size(other_bytes),
        files: 0,
        meta: "System, updates, logs, temporary files".to_string(),
        detail: "Other Storage includes the operating system, update files, logs, and anything not classified as games, artwork, or AI models.".to_string(),
        percent_of_total: percent(other_bytes, total_bytes),
    };
    let warning_copy = if percent_used >= 90 {
        "Storage is almost full. Sync, updates, and AI model loading may fail until space is freed."
    } else {
        "The console has enough free space for games, artwork, updates, and local AI models."
    };
    StorageStatus {
        total_bytes,
        used_bytes,
        free_bytes,
        percent_used,
        health,
        header_state,
        header_class,
        header_tooltip: if percent_used >= 90 {
            format!(
                "Storage is low. Open Storage to free space. {} used.",
                percent_used
            )
        } else {
            format!("Storage is {}% used.", percent_used)
        },
        ok_copy:
            "The console has enough free space for games, artwork, updates, and local AI models.",
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

fn root_disk_usage() -> Option<(u64, u64, u64)> {
    let output = Command::new("df").args(["-B1", "/"]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.lines().nth(1)?;
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 5 {
        return None;
    }
    let total = parts.get(1)?.parse().ok()?;
    let used = parts.get(2)?.parse().ok()?;
    let free = parts.get(3)?.parse().ok()?;
    Some((total, used, free))
}

fn storage_health(percent_used: u8) -> (&'static str, String, &'static str) {
    match percent_used {
        0..=74 => ("OK", "OK".to_string(), "good"),
        75..=89 => ("Getting Full", format!("{}%", percent_used), "warn"),
        90..=97 => ("Low Space", "Low".to_string(), "warn"),
        _ => ("Full", "Full".to_string(), "bad"),
    }
}

fn game_storage(total_bytes: u64) -> StorageCategoryStatus {
    let mut seen = Vec::<PathBuf>::new();
    let mut folders = Vec::<(String, u64)>::new();
    let mut bytes = 0u64;
    let mut files = 0u64;
    for path in game_candidate_paths() {
        if !path.exists() || seen.iter().any(|prior| path.starts_with(prior)) {
            continue;
        }
        let usage = path_usage(&path);
        if usage.bytes > 0 || usage.files > 0 {
            let name = path
                .file_name()
                .and_then(|v| v.to_str())
                .unwrap_or("games")
                .to_string();
            folders.push((name, usage.bytes));
            bytes = bytes.saturating_add(usage.bytes);
            files = files.saturating_add(usage.files);
            seen.push(path);
        }
    }
    folders.sort_by(|a, b| b.1.cmp(&a.1));
    let largest = folders
        .iter()
        .take(7)
        .map(|(name, _)| name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    StorageCategoryStatus {
        bytes,
        size: human_size(bytes),
        files,
        meta: format!("{} files", files),
        detail: if largest.is_empty() {
            "No copied game files were found in the game folders.".to_string()
        } else {
            format!("Largest folders: {}.", largest)
        },
        percent_of_total: percent(bytes, total_bytes),
    }
}

fn game_candidate_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for system in GAME_SYSTEMS {
        paths.push(Path::new(GAMES_ROOT).join(system));
        paths.push(Path::new(GAMES_ROOT).join("roms").join(system));
        paths.push(Path::new(GAMES_ROOT).join("isos").join(system));
    }
    paths.push(Path::new(GAMES_ROOT).join("pc").join("dos"));
    paths
}

fn category_from_path(
    path: &Path,
    total_bytes: u64,
    meta_suffix: &str,
    fallback_detail: &str,
) -> StorageCategoryStatus {
    let usage = path_usage(path);
    StorageCategoryStatus {
        bytes: usage.bytes,
        size: human_size(usage.bytes),
        files: usage.files,
        meta: format!("{} {}", usage.files, meta_suffix),
        detail: fallback_detail.to_string(),
        percent_of_total: percent(usage.bytes, total_bytes),
    }
}

fn ai_model_storage(total_bytes: u64) -> AiModelStorageStatus {
    let mut models = Vec::new();
    for root in MODEL_SCAN_ROOTS {
        collect_ai_models(Path::new(root), &mut models, 0);
    }
    models.sort_by(|a, b| b.0.cmp(&a.0));
    let total = models.iter().map(|(size, _, _)| *size).sum::<u64>();
    let rows = models
        .into_iter()
        .map(|(size, filename, _path)| AiModelDiskStatus {
            friendly_name: friendly_model_name(&filename),
            filename,
            size: human_size(size),
            status: "Available",
        })
        .collect::<Vec<_>>();
    let count = rows.len();
    AiModelStorageStatus {
        bytes: total,
        size: human_size(total),
        count,
        meta: format!("{} installed models", count),
        detail: if count == 0 {
            "No local AI model files were found on console storage.".to_string()
        } else {
            "Model files are stored locally for Local AI. Remove unused models to free space without affecting games.".to_string()
        },
        percent_of_total: percent(total, total_bytes),
        models: rows,
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
    for root in MODEL_SCAN_ROOTS {
        collect_ai_models(Path::new(root), &mut models, 0);
    }
    models
        .into_iter()
        .find(|(_, name, _)| name == filename)
        .map(|(_, _, path)| path)
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

fn human_size(bytes: u64) -> String {
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
    fn sync_view_is_ceremonial_workflow_not_log_first() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://console.home.arpa/".to_string(),
            product: "HomeConsole".to_string(),
        };
        let status = console_status(&state);
        let rendered = ui::layout(&status).into_string();
        let sync_start = rendered.find("id=\"view-sync\"").expect("sync view starts");
        let sync_end = rendered
            .find("id=\"view-storage\"")
            .expect("storage follows sync");
        let sync_html = &rendered[sync_start..sync_end];

        for required in [
            "Sync Games",
            "Sync scans the console’s game folders, finds copied games, fetches artwork, and adds playable entries to GameScope.",
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
            "View Sync Log",
            "Metadata keys are optional. They improve artwork and titles, but games can still sync without them.",
            "SteamGridDB",
            "TheGamesDB",
            "ScreenScraper",
        ] {
            assert!(sync_html.contains(required), "missing {required}");
        }

        let workflow = sync_html.find("sync-workflow").expect("workflow shown");
        let log = sync_html.find("sync-log-panel").expect("log available");
        assert!(workflow < log, "workflow appears before the collapsed log");
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
            "Clean Temporary Files",
            "data-nav-target=\"storage\"",
        ] {
            assert!(rendered.contains(required), "missing {required}");
        }
        assert!(
            rendered.contains("Remove Model") || rendered.contains("No local AI model files"),
            "storage page must either show removable models or the true empty model state"
        );
        if status.storage.percent_used >= 90 {
            assert!(rendered.contains(
                "Storage is low. Sync may fail if artwork or shortcuts cannot be written."
            ));
            assert!(rendered.contains(
                "Storage is low. Remove unused games, artwork, or AI models before adding more models."
            ));
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
    fn home_onboarding_and_network_view_support_first_run_setup() {
        let state = AppState {
            started_unix: 0,
            canonical_url: "http://arcadia.home.arpa/".to_string(),
            product: "HomeConsole".to_string(),
        };
        let status = console_status(&state);
        let rendered = ui::layout(&status).into_string();
        let home_start = rendered.find("id=\"view-home\"").expect("home view starts");
        let home_end = rendered
            .find("id=\"view-games\"")
            .expect("games follows home");
        let home_html = &rendered[home_start..home_end];
        let onboarding = home_html
            .find("Welcome to Arcadia")
            .expect("onboarding card shown");
        let actions = home_html
            .find("What do you want to do?")
            .expect("home actions shown");
        assert!(
            onboarding < actions,
            "onboarding appears above normal Home actions"
        );
        for required in [
            "Welcome to Arcadia",
            "Finish these steps to set up your local game console.",
            "Connect to Network",
            "Add Games",
            "Run First Sync",
            "Start Playing",
            "Not Started",
            "Ready",
            "Connect Arcadia to your home network so other devices can copy games to it and use Local AI.",
            "Open Network Settings",
            "data-nav-target=\"network\"",
            "Connected by Ethernet",
            "Connected to Wi-Fi",
            "No network connection",
            "Copy games into Arcadia’s network folders from another device on your home network.",
            "Sync turns copied files into playable GameScope entries with artwork and titles when available.",
            "Return to Console",
            "Hide for now",
        ] {
            assert!(home_html.contains(required), "missing onboarding {required}");
        }

        assert!(rendered.contains("data-view=\"network\""));
        let network_start = rendered
            .find("id=\"view-network\"")
            .expect("network view starts");
        let network_end = rendered
            .find("id=\"view-access-pin\"")
            .expect("access follows network");
        let network_html = &rendered[network_start..network_end];
        for required in [
            "Network",
            "Manage how Arcadia connects to your home network. Network access is required for copying games, using the web console, and LAN inference.",
            "Connection Status",
            "Connected by Ethernet",
            "Status",
            "Connected",
            "Active connection",
            "Ethernet",
            "Network name",
            "IP address",
            "Signal strength",
            "Internet reachability",
            "Not Checked",
            "Wi-Fi",
            "Wi-Fi adapter status",
            "Enabled",
            "Current SSID",
            "Security type",
            "Saved networks",
            "Scan for Networks",
            "Connect",
            "Disconnect",
            "Forget Network",
            "Show Saved Networks",
            "Hidden network name",
            "Wi-Fi password",
            "Show while typing",
            "Could not connect to this Wi-Fi network. Check the password and try again.",
            "Wi-Fi signal is weak. Move Arcadia closer to the router or use Ethernet.",
            "Ethernet",
            "Ethernet status",
            "Link speed",
            "MAC address",
            "Ethernet is recommended for large game transfers and stable LAN inference.",
            "Device Addresses",
            "http://arcadia.home.arpa",
            "\\\\ARCADIA",
            "smb://ARCADIA",
            "http://arcadia.home.arpa:7777",
            "Network Services",
            "Web Console",
            "Games Folder",
            "LAN Inference",
            "SSH",
        ] {
            assert!(network_html.contains(required), "missing network {required}");
        }
        for forbidden in ["nmcli", "iwctl", "ip addr", "saved password"] {
            assert!(
                !network_html.contains(forbidden),
                "network leaked raw/secret term: {forbidden}"
            );
        }
        assert!(rendered.contains("data-nav-target=\"network\""));
        assert!(APP_JS.contains("localStorage.setItem('onboarding.firstSyncComplete', 'true')"));
        assert!(APP_JS.contains("sessionStorage.setItem('onboarding.hideForNow', 'true')"));
        assert!(APP_JS.contains(
            "Could not connect to this Wi-Fi network. Check the password and try again."
        ));
        assert!(APP_JS.contains("input.type = toggle.checked ? 'text' : 'password'"));
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
        for required in [
            "System",
            "View technical console status, service health, networking details, SSH access, and logs.",
            "SSH",
            "SSH status",
            "Disabled",
            "Hostname",
            "LAN IP address",
            "Username",
            "ssh console@console.home.arpa",
            "SSH is for direct technical access to the console. Normal game management does not require SSH.",
            "Only enable SSH on a trusted home network. Use a strong password or key-based access.",
            "Enable SSH",
            "Disable SSH",
            "Copy SSH Command",
            "Services",
            "GameScope",
            "Runs the console gaming session.",
            "Samba",
            "Shares game folders over the home network.",
            "Game Sync",
            "Adds copied games to the GameScope library.",
            "Local AI",
            "Loads the selected local AI model.",
            "LAN Inference",
            "Lets other home-network devices use Local AI.",
            "Web GUI",
            "Runs this management interface.",
            "Restart",
            "View Logs",
            "Logs",
            "Logs help diagnose problems. They are mostly useful for support or technical users.",
            "Sync Log",
            "Local AI Log",
            "LAN Inference Log",
            "System Log",
            "Web GUI Log",
            "View",
            "Copy",
            "Download",
            "Networking",
            "Local domain/path",
            "MAC address",
            "Network status",
            "Active interface",
            "Open local ports",
            "Web GUI",
            "http://console.home.arpa",
            "Games Folder",
            "\\\\HOMECONSOLE",
            "http://console.home.arpa:7777",
            "80/443",
            "445",
            "7777",
            "22",
            "LAN Inference is intended only for trusted home networks. Do not expose port 7777 to the public internet.",
        ] {
            assert!(system_html.contains(required), "missing {required}");
        }

        let logs = system_html.find("Logs").expect("logs section shown");
        let first_log_group = system_html.find("Sync Log").expect("sync log group shown");
        assert!(
            logs < first_log_group,
            "logs are structured below section intro"
        );
        assert!(rendered.contains("data-nav-target=\"system\""));
        assert!(APP_JS.contains("if (view === 'advanced') view = 'system';"));
        assert!(APP_JS.contains("Restarting GameScope may close the active game session."));
        assert!(!rendered.contains("data-view=\"advanced\""));
        assert!(!rendered.contains(">Advanced<"));
        for forbidden in ["Expert Mode", "Developer"] {
            assert!(
                !rendered.contains(forbidden),
                "forbidden label survived: {forbidden}"
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
            "view-network",
            "view-access-pin",
            "view-updates",
            "view-power",
            "view-system",
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
