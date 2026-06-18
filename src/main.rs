use axum::{
    extract::State,
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
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
const VAULT_MOUNTPOINT: &str = "/vault";
const VAULT_STATE_PATH: &str = "/var/lib/homeconsole/state.json";
const VAULT_UNLOCK_HELPER: &str = "/usr/local/sbin/homeconsole-vault-unlock";
const VAULT_PASSWORD_CHANGE_HELPER: &str = "/usr/local/sbin/homeconsole-vault-password-change";
const VAULT_PASSWORD_RESET_HELPER: &str =
    "/usr/local/sbin/homeconsole-vault-password-reset-default";

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
    pub vault: VaultStatus,
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
pub struct VaultStatus {
    pub mounted: bool,
    pub mountpoint: &'static str,
    pub state_path: &'static str,
    pub mapper_present: bool,
    pub unlock_helper_present: bool,
    pub password_change_helper_present: bool,
    pub password_reset_helper_present: bool,
    pub default_reset_available: bool,
    pub first_gate: &'static str,
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
struct UnlockRequest {
    password: String,
}

#[derive(Deserialize)]
struct PasswordChangeRequest {
    current_password: String,
    new_password: String,
}

#[derive(Deserialize)]
struct PasswordResetRequest {
    confirm: String,
}

#[derive(Serialize)]
struct VaultActionResponse {
    ok: bool,
    mounted: bool,
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
        .route("/api/vault/status", get(vault_status_route))
        .route("/api/vault/password/change", post(change_vault_password))
        .route(
            "/api/vault/password/reset-default",
            post(reset_vault_password_default),
        )
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

async fn vault_status_route() -> Json<VaultStatus> {
    Json(vault_status())
}

async fn pre_unlock(Json(body): Json<UnlockRequest>) -> (StatusCode, Json<VaultActionResponse>) {
    if body.password.is_empty() {
        return action_response(
            StatusCode::BAD_REQUEST,
            false,
            false,
            "unlock",
            helper_exists(VAULT_UNLOCK_HELPER),
            "Vault password is required.",
        );
    }

    run_vault_helper(
        VAULT_UNLOCK_HELPER,
        "unlock",
        &[body.password.as_str()],
        "Vault unlocked.",
        "Vault unlock failed.",
    )
}

async fn change_vault_password(
    Json(body): Json<PasswordChangeRequest>,
) -> (StatusCode, Json<VaultActionResponse>) {
    if body.current_password.is_empty() || body.new_password.is_empty() {
        return action_response(
            StatusCode::BAD_REQUEST,
            false,
            vault_status().mounted,
            "change-password",
            helper_exists(VAULT_PASSWORD_CHANGE_HELPER),
            "Current password and new password are required.",
        );
    }
    if body.new_password.len() < 4 {
        return action_response(
            StatusCode::BAD_REQUEST,
            false,
            vault_status().mounted,
            "change-password",
            helper_exists(VAULT_PASSWORD_CHANGE_HELPER),
            "New password is too short.",
        );
    }

    run_vault_helper(
        VAULT_PASSWORD_CHANGE_HELPER,
        "change-password",
        &[body.current_password.as_str(), body.new_password.as_str()],
        "Vault password changed.",
        "Vault password change failed.",
    )
}

async fn reset_vault_password_default(
    Json(body): Json<PasswordResetRequest>,
) -> (StatusCode, Json<VaultActionResponse>) {
    if body.confirm != "RESET" {
        return action_response(
            StatusCode::BAD_REQUEST,
            false,
            vault_status().mounted,
            "reset-default-password",
            helper_exists(VAULT_PASSWORD_RESET_HELPER),
            "Type RESET to restore the default vault password.",
        );
    }

    run_vault_helper(
        VAULT_PASSWORD_RESET_HELPER,
        "reset-default-password",
        &[],
        "Vault password reset to the appliance default.",
        "Vault password reset failed.",
    )
}

fn run_vault_helper(
    helper: &'static str,
    action: &'static str,
    secret_lines: &[&str],
    success_message: &str,
    failure_message: &str,
) -> (StatusCode, Json<VaultActionResponse>) {
    if !helper_exists(helper) {
        return action_response(
            StatusCode::NOT_IMPLEMENTED,
            false,
            vault_status().mounted,
            action,
            false,
            "Vault password helper is not installed on this unit yet.",
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
            return action_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                false,
                vault_status().mounted,
                action,
                true,
                "Failed to start vault password helper.",
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
    action_response(
        if ok {
            StatusCode::OK
        } else {
            StatusCode::FORBIDDEN
        },
        ok,
        vault_status().mounted,
        action,
        true,
        if ok { success_message } else { failure_message },
    )
}

fn action_response(
    status: StatusCode,
    ok: bool,
    mounted: bool,
    action: &'static str,
    helper_present: bool,
    message: &str,
) -> (StatusCode, Json<VaultActionResponse>) {
    (
        status,
        Json(VaultActionResponse {
            ok,
            mounted,
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
        schema: "arcadia.status.v5",
        product: state.product.clone(),
        canonical_url: state.canonical_url.clone(),
        arcadia: ArcadiaStatus {
            service: "arcadia",
            version: env!("CARGO_PKG_VERSION"),
            mode: "single-pane-intent-modules",
            ui: "header-plus-single-pane-fractal-intent-modules",
        },
        runtime: runtime_status(state.started_unix),
        vault: vault_status(),
        surfaces: SurfaceStatus {
            http: "console.home.arpa -> :8080",
            mdns: "homeconsole.local",
            smb: "HOMECONSOLE",
        },
        ui_contract: UiContract {
            schema: "arcadia.ui.contract.v3",
            button_variants: ["primary", "secondary", "danger"],
            composition: "header -> one single console pane -> ordered fractal intent modules; modules may contain nested button, text, status, or form bricks",
            modal: "confirmation/readback only; password mutation posts to local root-owned helpers",
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

fn vault_status() -> VaultStatus {
    let reset_helper_present = helper_exists(VAULT_PASSWORD_RESET_HELPER);
    VaultStatus {
        mounted: is_mountpoint(VAULT_MOUNTPOINT),
        mountpoint: VAULT_MOUNTPOINT,
        state_path: VAULT_STATE_PATH,
        mapper_present: Path::new("/dev/mapper/homeconsole-vault").exists(),
        unlock_helper_present: helper_exists(VAULT_UNLOCK_HELPER),
        password_change_helper_present: helper_exists(VAULT_PASSWORD_CHANGE_HELPER),
        password_reset_helper_present: reset_helper_present,
        default_reset_available: reset_helper_present,
        first_gate: "vault-status-before-console-dashboard",
    }
}

fn helper_exists(path: &str) -> bool {
    Path::new(path).exists()
}

fn is_mountpoint(path: &str) -> bool {
    fs::read_to_string("/proc/mounts")
        .map(|mounts| {
            mounts
                .lines()
                .any(|line| line.split_whitespace().nth(1) == Some(path))
        })
        .unwrap_or(false)
}

mod anyhow_free {
    pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
}
