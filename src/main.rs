use axum::{
    extract::State,
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use maud::{html, Markup, DOCTYPE};
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
struct Health {
    ok: bool,
    service: &'static str,
    product: String,
    version: &'static str,
    started_unix: u64,
}

#[derive(Clone, Serialize)]
struct ConsoleStatus {
    schema: &'static str,
    product: String,
    canonical_url: String,
    arcadia: ArcadiaStatus,
    vault: VaultStatus,
    portals: Vec<Portal>,
    surfaces: SurfaceStatus,
}

#[derive(Clone, Serialize)]
struct ArcadiaStatus {
    service: &'static str,
    version: &'static str,
    mode: &'static str,
    ui: &'static str,
}

#[derive(Clone, Serialize)]
struct VaultStatus {
    mounted: bool,
    mountpoint: &'static str,
    state_path: &'static str,
    mapper_present: bool,
    unlock_helper_present: bool,
    first_gate: &'static str,
}

#[derive(Clone, Serialize)]
struct SurfaceStatus {
    http: &'static str,
    mdns: &'static str,
    smb: &'static str,
}

#[derive(Clone, Serialize)]
struct Portal {
    name: &'static str,
    description: &'static str,
    local_url: &'static str,
    status: &'static str,
    icon: &'static str,
    action: &'static str,
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

async fn index(State(state): State<Arc<AppState>>) -> Markup {
    let status = console_status(&state);
    ui::layout(&status)
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
        return (
            StatusCode::BAD_REQUEST,
            Json(UnlockResponse {
                ok: false,
                mounted: false,
                message: "Vault password is required.".to_string(),
            }),
        );
    }

    // The password is intentionally never logged and never persisted. Arcadia may only
    // pass it to a fixed root-owned helper when that membrane exists on the appliance.
    if !Path::new(VAULT_UNLOCK_HELPER).exists() {
        return (
            StatusCode::NOT_IMPLEMENTED,
            Json(UnlockResponse {
                ok: false,
                mounted: vault_status().mounted,
                message: "Vault unlock helper is not installed on this unit yet.".to_string(),
            }),
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
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(UnlockResponse {
                    ok: false,
                    mounted: vault_status().mounted,
                    message: "Failed to start vault unlock helper.".to_string(),
                }),
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
    (
        if ok && mounted {
            StatusCode::OK
        } else {
            StatusCode::FORBIDDEN
        },
        Json(UnlockResponse {
            ok: ok && mounted,
            mounted,
            message: if ok && mounted {
                "Vault unlocked."
            } else {
                "Vault unlock failed."
            }
            .to_string(),
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
        schema: "arcadia.status.v2",
        product: state.product.clone(),
        canonical_url: state.canonical_url.clone(),
        arcadia: ArcadiaStatus {
            service: "arcadia",
            version: env!("CARGO_PKG_VERSION"),
            mode: "portals-vault-gated",
            ui: "homeserver-portals-vaultauth-popup-manager",
        },
        vault: vault_status(),
        portals: portals(),
        surfaces: SurfaceStatus {
            http: "console.home.arpa -> :8080",
            mdns: "homeconsole.local",
            smb: "HOMECONSOLE",
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
        Portal {
            name: "Games",
            description: "Sync the runtime game library into Steam tiles.",
            local_url: "#",
            status: "unknown",
            icon: "🎮",
            action: "sync-games",
        },
        Portal {
            name: "Vault",
            description: "Unlock and inspect the encrypted HomeConsole vault.",
            local_url: "#",
            status: "partial",
            icon: "🔐",
            action: "vault-status",
        },
        Portal {
            name: "Files",
            description: "Open the HOMECONSOLE SMB file surface.",
            local_url: "smb://HOMECONSOLE",
            status: "up",
            icon: "🗂️",
            action: "open-files",
        },
        Portal {
            name: "Updates",
            description: "Harmonia profile and Arcadia update membrane.",
            local_url: "#",
            status: "unknown",
            icon: "🛠️",
            action: "updates",
        },
        Portal {
            name: "Network",
            description: "LAN route, mDNS fallback, and service identity.",
            local_url: "/api/status",
            status: "up",
            icon: "🌐",
            action: "network",
        },
        Portal {
            name: "Receipts",
            description: "Source, runtime, and transition proof surfaces.",
            local_url: "#",
            status: "unknown",
            icon: "🧾",
            action: "receipts",
        },
    ]
}

mod ui {
    use super::*;

    pub fn layout(status: &ConsoleStatus) -> Markup {
        html! {
            (DOCTYPE)
            html lang="en" {
                head {
                    meta charset="utf-8";
                    meta name="viewport" content="width=device-width, initial-scale=1";
                    title { (status.product) " / Arcadia" }
                    link rel="stylesheet" href="/static/app.css";
                }
                body data-vault-mounted=(status.vault.mounted) {
                    div id="vault-gate" class="vault-auth-container" data-mounted=(status.vault.mounted) {
                        (vault_auth(status))
                    }
                    div id="app" class="portals-tablet" aria-hidden=(!status.vault.mounted) {
                        header class="portal-page-header" {
                            div {
                                h1 { "Arcadia" }
                                p { "HomeConsole portals" }
                            }
                            button class="small-status-button" data-modal-title="Vault status" data-modal-body=(vault_modal_text(status)) { "Vault status" }
                        }
                        main class="portals-grid" {
                            @for portal in &status.portals {
                                (portal_card(portal))
                            }
                        }
                    }
                    (popup_root())
                    script src="/static/app.js" {}
                }
            }
        }
    }

    fn vault_auth(status: &ConsoleStatus) -> Markup {
        html! {
            div class="vault-auth-card" {
                div class="vault-auth-logo" aria-hidden="true" { "A" }
                h1 { "HomeConsole" }
                @if status.vault.mounted {
                    h2 { "Vault Mounted" }
                    p class="vault-auth-desc" { "The vault is unlocked. Opening Arcadia portals." }
                } @else {
                    h2 { "Vault Authentication" }
                    p class="vault-auth-desc" { "Please enter your vault password to continue." }
                    form id="vault-unlock-form" class="vault-auth-form" autocomplete="off" {
                        div class="form-group" {
                            input class="vault-auth-input vault-auth-input-password" type="password" name="password" placeholder="Enter vault password" autocomplete="current-password" autofocus;
                        }
                        div id="vault-auth-error" class="vault-auth-error" hidden {}
                        button class="vault-auth-button" type="submit" { "Unlock Vault" }
                    }
                }
                small { "Product of HOMESERVER LLC" }
                div class="version-info" { small { "Version " (env!("CARGO_PKG_VERSION")) " / " (status.arcadia.mode) } }
            }
        }
    }

    fn portal_card(portal: &Portal) -> Markup {
        html! {
            div class=(format!("portal-card {}", portal.status))
                role="button" tabindex="0"
                data-action=(portal.action)
                data-url=(portal.local_url)
                data-modal-title=(portal.name)
                data-modal-body=(portal.description) {
                div class="portal-card-header" {
                    div class="portal-icon" aria-hidden="true" { (portal.icon) }
                    h3 class="portal-name" { (portal.name) }
                    p class="portal-description" { (portal.description) }
                }
            }
        }
    }

    fn popup_root() -> Markup {
        html! {
            div id="popup-root" data-popup-root="true" {
                div id="modal-overlay" class="modal-overlay" hidden {
                    div class="modal" role="dialog" aria-modal="true" aria-labelledby="modal-title" {
                        button id="modal-close" class="modal-close" type="button" aria-label="Close modal" { "×" }
                        h2 id="modal-title" class="modal-title" {}
                        div id="modal-content" class="modal-content" {}
                        div class="modal-buttons" { button id="modal-ok" type="button" { "OK" } }
                    }
                }
                div id="toast-container" class="toast-container" aria-live="polite" {}
            }
        }
    }

    fn vault_modal_text(status: &ConsoleStatus) -> String {
        format!(
            "mounted: {}\nmountpoint: {}\nmapper_present: {}\nunlock_helper_present: {}\nstate_path: {}",
            status.vault.mounted, status.vault.mountpoint, status.vault.mapper_present, status.vault.unlock_helper_present, status.vault.state_path
        )
    }
}

mod anyhow_free {
    pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
}
