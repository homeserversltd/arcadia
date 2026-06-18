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
    pub vault: VaultStatus,
    pub portals: Vec<Portal>,
    pub surfaces: SurfaceStatus,
    pub ui_contract: UiContract,
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
    pub first_gate: &'static str,
}

#[derive(Clone, Serialize)]
pub struct SurfaceStatus {
    pub http: &'static str,
    pub mdns: &'static str,
    pub smb: &'static str,
}

#[derive(Clone, Serialize)]
pub struct Portal {
    pub name: &'static str,
    pub description: &'static str,
    pub local_url: &'static str,
    pub status: PortalState,
    pub icon: &'static str,
    pub action: &'static str,
    pub density: Vec<TileDatum>,
    pub buttons: Vec<ButtonAction>,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PortalState {
    Up,
    Down,
    Partial,
    Unknown,
}
impl PortalState {
    pub fn class(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
            Self::Partial => "partial",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Serialize)]
pub struct TileDatum {
    pub label: &'static str,
    pub value: &'static str,
    pub state: PortalState,
}

#[derive(Clone, Serialize)]
pub struct ButtonAction {
    pub label: &'static str,
    pub variant: ButtonVariant,
    pub action: &'static str,
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

#[derive(Serialize)]
struct UnlockResponse {
    ok: bool,
    mounted: bool,
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

async fn pre_unlock(Json(body): Json<UnlockRequest>) -> (StatusCode, Json<UnlockResponse>) {
    if body.password.is_empty() {
        return response(
            StatusCode::BAD_REQUEST,
            false,
            false,
            "Vault password is required.",
        );
    }
    if !Path::new(VAULT_UNLOCK_HELPER).exists() {
        return response(
            StatusCode::NOT_IMPLEMENTED,
            false,
            vault_status().mounted,
            "Vault unlock helper is not installed on this unit yet.",
        );
    }

    let mut child = match Command::new(VAULT_UNLOCK_HELPER)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => {
            return response(
                StatusCode::INTERNAL_SERVER_ERROR,
                false,
                vault_status().mounted,
                "Failed to start vault unlock helper.",
            )
        }
    };

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        let _ = stdin.write_all(body.password.as_bytes());
        let _ = stdin.write_all(b"\n");
    }

    let ok = child.wait().map(|s| s.success()).unwrap_or(false);
    let mounted = vault_status().mounted;
    response(
        if ok && mounted {
            StatusCode::OK
        } else {
            StatusCode::FORBIDDEN
        },
        ok && mounted,
        mounted,
        if ok && mounted {
            "Vault unlocked."
        } else {
            "Vault unlock failed."
        },
    )
}

fn response(
    status: StatusCode,
    ok: bool,
    mounted: bool,
    message: &str,
) -> (StatusCode, Json<UnlockResponse>) {
    (
        status,
        Json(UnlockResponse {
            ok,
            mounted,
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
        schema: "arcadia.status.v3",
        product: state.product.clone(),
        canonical_url: state.canonical_url.clone(),
        arcadia: ArcadiaStatus {
            service: "arcadia",
            version: env!("CARGO_PKG_VERSION"),
            mode: "fractal-portals-vault-gated",
            ui: "centralized-tile-pane-modal-schema",
        },
        vault: vault_status(),
        portals: portals(),
        surfaces: SurfaceStatus {
            http: "console.home.arpa -> :8080",
            mdns: "homeconsole.local",
            smb: "HOMECONSOLE",
        },
        ui_contract: UiContract {
            schema: "arcadia.ui.contract.v1",
            button_variants: ["primary", "secondary", "danger"],
            composition: "button -> tile -> pane -> nested tile -> singleton modal pane",
            modal: "one modal root receives every tile/button press",
        },
    }
}

fn vault_status() -> VaultStatus {
    VaultStatus {
        mounted: is_mountpoint(VAULT_MOUNTPOINT),
        mountpoint: VAULT_MOUNTPOINT,
        state_path: VAULT_STATE_PATH,
        mapper_present: Path::new("/dev/mapper/homeconsole-vault").exists(),
        unlock_helper_present: Path::new(VAULT_UNLOCK_HELPER).exists(),
        first_gate: "vault-status-before-portals",
    }
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

fn portals() -> Vec<Portal> {
    vec![
        portal(
            "Games",
            "Runtime library and Steam tile sync.",
            "#",
            PortalState::Unknown,
            "🎮",
            "sync-games",
            [
                ("folders", "12 lanes", PortalState::Up),
                ("sync", "declared", PortalState::Unknown),
                ("payloads", "runtime", PortalState::Partial),
            ],
        ),
        portal(
            "Vault",
            "Encrypted HomeConsole vault membrane.",
            "#",
            PortalState::Partial,
            "🔐",
            "vault-status",
            [
                ("gate", "first", PortalState::Up),
                ("policy", "manual", PortalState::Partial),
                ("helper", "pending", PortalState::Unknown),
            ],
        ),
        portal(
            "Files",
            "HOMECONSOLE file/drop surface.",
            "smb://HOMECONSOLE",
            PortalState::Up,
            "🗂️",
            "open-files",
            [
                ("smb", "HOMECONSOLE", PortalState::Up),
                ("drops", "direct", PortalState::Up),
                ("source", "runtime", PortalState::Partial),
            ],
        ),
        portal(
            "Updates",
            "Harmonia profile and Arcadia artifact membrane.",
            "#",
            PortalState::Unknown,
            "🛠️",
            "updates",
            [
                ("engine", "harmonia", PortalState::Partial),
                ("profile", "homeconsole", PortalState::Up),
                ("bridge", "manual", PortalState::Unknown),
            ],
        ),
        portal(
            "Network",
            "LAN route, mDNS fallback, service identity.",
            "/api/status",
            PortalState::Up,
            "🌐",
            "network",
            [
                ("dns", "console.home.arpa", PortalState::Up),
                ("mdns", "homeconsole.local", PortalState::Up),
                ("port", "80→8080", PortalState::Up),
            ],
        ),
        portal(
            "Receipts",
            "Source/runtime proof and transition readbacks.",
            "#",
            PortalState::Unknown,
            "🧾",
            "receipts",
            [
                ("source", "cibation", PortalState::Up),
                ("runtime", "systemd+curl", PortalState::Up),
                ("admit", "operator", PortalState::Unknown),
            ],
        ),
    ]
}

fn portal(
    name: &'static str,
    description: &'static str,
    local_url: &'static str,
    status: PortalState,
    icon: &'static str,
    action: &'static str,
    density: [(&'static str, &'static str, PortalState); 3],
) -> Portal {
    Portal {
        name,
        description,
        local_url,
        status,
        icon,
        action,
        density: density
            .into_iter()
            .map(|(label, value, state)| TileDatum {
                label,
                value,
                state,
            })
            .collect(),
        buttons: vec![
            ButtonAction {
                label: "Open",
                variant: ButtonVariant::Primary,
                action,
            },
            ButtonAction {
                label: "Details",
                variant: ButtonVariant::Secondary,
                action: "details",
            },
            ButtonAction {
                label: "Hold",
                variant: ButtonVariant::Danger,
                action: "hold",
            },
        ],
    }
}

mod anyhow_free {
    pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
}
