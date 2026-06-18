use axum::{
    extract::State,
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use maud::{html, Markup, DOCTYPE};
use serde::Serialize;
use std::{
    env,
    net::SocketAddr,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

const APP_CSS: &str = include_str!("../static/app.css");
const APP_JS: &str = include_str!("../static/app.js");

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
    games: GamesStatus,
    update: UpdateStatus,
    surfaces: SurfaceStatus,
    receipts: ReceiptStatus,
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
    policy: &'static str,
    state_path: &'static str,
    unlock_surface: &'static str,
}

#[derive(Clone, Serialize)]
struct GamesStatus {
    intake: &'static str,
    sync_command: &'static str,
    payload_boundary: &'static str,
}

#[derive(Clone, Serialize)]
struct UpdateStatus {
    engine: &'static str,
    profile: &'static str,
    arcadia_transition: &'static str,
}

#[derive(Clone, Serialize)]
struct SurfaceStatus {
    http: &'static str,
    mdns: &'static str,
    smb: &'static str,
}

#[derive(Clone, Serialize)]
struct ReceiptStatus {
    source: &'static str,
    runtime: &'static str,
    update: &'static str,
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
        .route("/fragments/receipt", get(receipt_fragment))
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
    ui::layout(&status, dashboard(&status))
}

async fn health(State(state): State<Arc<AppState>>) -> Json<Health> {
    Json(health_status(&state))
}

async fn status(State(state): State<Arc<AppState>>) -> Json<ConsoleStatus> {
    Json(console_status(&state))
}

async fn receipt_fragment(State(state): State<Arc<AppState>>) -> Markup {
    ui::receipt_panel(
        &console_status(&state),
        "Arcadia GUI rendered from Rust components.",
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

fn health_status(state: &AppState) -> Health {
    Health {
        ok: true,
        service: "arcadia",
        product: state.product.clone(),
        version: env!("CARGO_PKG_VERSION"),
        started_unix: state.started_unix,
    }
}

fn console_status(state: &AppState) -> ConsoleStatus {
    ConsoleStatus {
        schema: "arcadia.status.v1",
        product: state.product.clone(),
        canonical_url: state.canonical_url.clone(),
        arcadia: ArcadiaStatus {
            service: "arcadia",
            version: env!("CARGO_PKG_VERSION"),
            mode: "homeserver-ui-maud",
            ui: "maud-components-home-server-class-grammar",
        },
        vault: VaultStatus {
            policy: "manual-unlock-default",
            state_path: "/var/lib/homeconsole/state.json",
            unlock_surface: "arcadia-vault-panel-next",
        },
        games: GamesStatus {
            intake: "samba-direct-folders",
            sync_command: "/usr/local/bin/arch-game-sync",
            payload_boundary: "runtime-only",
        },
        update: UpdateStatus {
            engine: "harmonia",
            profile: "homeconsole",
            arcadia_transition: "manual-bridge-now-harmonia-next",
        },
        surfaces: SurfaceStatus {
            http: "0.0.0.0:8080 behind LAN port 80 redirect",
            mdns: "homeconsole.local",
            smb: "HOMECONSOLE",
        },
        receipts: ReceiptStatus {
            source: "cibation",
            runtime: "systemd+curl",
            update: "/var/lib/harmonia/receipts/arcadia-latest/run.json",
        },
    }
}

fn dashboard(status: &ConsoleStatus) -> Markup {
    html! {
        section class="hero-panel ui-card ui-card--active" {
            div class="hero-copy" {
                div class="eyebrow" { "HomeConsole Arcadia" }
                h1 { "Providence Interface" }
                p class="lede" {
                    "A Rust-native control surface quarrying the Home Server UI language into a console appliance: router, NAS, and game console in one luminous membrane."
                }
            }
            div class="hero-actions" {
                (ui::action_link("Health", "/health", ui::ButtonVariant::Secondary))
                (ui::action_link("Status JSON", "/api/status", ui::ButtonVariant::Primary))
            }
        }

        section class="status-grid" aria-label="HomeConsole status" {
            (ui::status_card("System", ui::BadgeVariant::Success, "Arcadia active", "Rust axum + Maud renders the GUI on the console itself.", "🜂"))
            (ui::status_card("Vault", ui::BadgeVariant::Warning, status.vault.policy, "Encrypted /vault is born locked by default; unlock UI is the next blessed action.", "🔐"))
            (ui::status_card("Games", ui::BadgeVariant::Info, status.games.intake, "Drop games into SMB folders and press the sync transition when ready.", "🎮"))
            (ui::status_card("Network", ui::BadgeVariant::Success, "console.home.arpa", "LAN HTTP routes to the unprivileged backend on :8080.", "🌐"))
        }

        section class="console-columns" {
            (ui::action_card("Game library", "Synchronize games", "Run the declared arch-game-sync transition and return a receipt.", "/actions/sync-games", "#receipt-panel", ui::ButtonVariant::Primary))
            (ui::action_card("Vault", "Unlock vault", "Prepare the manual unlock flow without logging raw secrets.", "/actions/vault-unlock", "#receipt-panel", ui::ButtonVariant::Warning))
            (ui::action_card("Harmonia", "Check updates", "Make the HomeConsole profile modern through the update manager.", "/actions/update-check", "#receipt-panel", ui::ButtonVariant::Secondary))
        }

        section class="row-stack" aria-label="Service surfaces" {
            (ui::row_info_tile("HTTP", &status.canonical_url, "LAN route", &[ui::badge(ui::BadgeVariant::Success, "up")], "🛰️"))
            (ui::row_info_tile("mDNS", status.surfaces.mdns, "Fallback discovery", &[ui::badge(ui::BadgeVariant::Info, "discoverable")], "📡"))
            (ui::row_info_tile("SMB", status.surfaces.smb, "Direct game folders", &[ui::badge(ui::BadgeVariant::Secondary, "runtime payloads")], "🗂️"))
            (ui::row_info_tile("Update", status.update.engine, "Harmonia profile spine", &[ui::badge(ui::BadgeVariant::Warning, "manual bridge")], "🛠️"))
        }

        div id="receipt-panel" {
            (ui::receipt_panel(status, "Awaiting the next exact transition."))
        }
    }
}

mod ui {
    use super::*;

    #[derive(Clone, Copy)]
    pub enum ButtonVariant {
        Primary,
        Secondary,
        Warning,
    }
    impl ButtonVariant {
        fn class(self) -> &'static str {
            match self {
                Self::Primary => "primary",
                Self::Secondary => "secondary",
                Self::Warning => "warning",
            }
        }
    }

    #[derive(Clone, Copy)]
    pub enum BadgeVariant {
        Secondary,
        Success,
        Warning,
        Info,
    }
    impl BadgeVariant {
        fn class(self) -> &'static str {
            match self {
                Self::Secondary => "secondary",
                Self::Success => "success",
                Self::Warning => "warning",
                Self::Info => "info",
            }
        }
    }

    pub fn layout(status: &ConsoleStatus, body: Markup) -> Markup {
        html! {
            (DOCTYPE)
            html lang="en" {
                head {
                    meta charset="utf-8";
                    meta name="viewport" content="width=device-width, initial-scale=1";
                    title { (status.product) " / Arcadia" }
                    link rel="stylesheet" href="/static/app.css";
                }
                body {
                    div class="app-shell" {
                        header class="app-header" {
                            div class="brand-cluster" {
                                div class="brand-mark" { "A" }
                                div {
                                    div class="brand-title" { (status.product) }
                                    div class="brand-subtitle" { "Arcadia control surface" }
                                }
                            }
                            nav class="ui-tab-group" aria-label="Primary" {
                                (tab("Dashboard", true, "#"))
                                (tab("Vault", false, "#receipt-panel"))
                                (tab("Games", false, "#receipt-panel"))
                                (tab("Update", false, "#receipt-panel"))
                                (tab("System", false, "#receipt-panel"))
                            }
                        }
                        main class="content" {
                            (body)
                        }
                    }
                    script src="/static/app.js" {}
                }
            }
        }
    }

    pub fn tab(label: &str, active: bool, href: &str) -> Markup {
        let active_class = if active { " ui-tab--active" } else { "" };
        html! {
            a class=(format!("ui-tab{}", active_class)) href=(href) {
                span class="ui-tab__content" {
                    span class="ui-tab__label" { (label) }
                }
            }
        }
    }

    pub fn action_link(label: &str, href: &str, variant: ButtonVariant) -> Markup {
        html! { a class=(format!("ui-button ui-button--{} ui-button--large", variant.class())) href=(href) { (label) } }
    }

    pub fn badge(variant: BadgeVariant, label: &str) -> Markup {
        html! { span class=(format!("ui-badge ui-badge--{} ui-badge--medium ui-badge--pill", variant.class())) { (label) } }
    }

    pub fn status_card(
        title: &str,
        variant: BadgeVariant,
        value: &str,
        detail: &str,
        icon: &str,
    ) -> Markup {
        html! {
            article class="ui-card status-card" {
                div class="ui-card__header status-card__head" {
                    span class="status-card__icon" { (icon) }
                    span { (title) }
                    (badge(variant, value))
                }
                div class="ui-card__body" {
                    h2 { (value) }
                    p { (detail) }
                }
            }
        }
    }

    pub fn action_card(
        title: &str,
        action: &str,
        detail: &str,
        endpoint: &str,
        target: &str,
        variant: ButtonVariant,
    ) -> Markup {
        html! {
            article class="ui-card ui-card--clickable action-card" {
                div class="ui-card__header" { (title) }
                div class="ui-card__body" {
                    p { (detail) }
                    button class=(format!("ui-button ui-button--{} ui-button--full-width", variant.class()))
                        hx-post=(endpoint) hx-target=(target) hx-swap="innerHTML" disabled {
                        (action)
                    }
                }
                div class="ui-card__footer" { "Transition endpoint reserved; no arbitrary shell." }
            }
        }
    }

    pub fn row_info_tile(
        title: &str,
        subtitle: &str,
        metadata: &str,
        badges: &[Markup],
        icon: &str,
    ) -> Markup {
        html! {
            div class="ui-row-info-tile" role="button" tabindex="0" {
                div class="ui-row-info-tile__icon" { (icon) }
                div class="ui-row-info-tile__content" {
                    div class="ui-row-info-tile__title" { (title) }
                    div class="ui-row-info-tile__subtitle" { (subtitle) }
                    div class="ui-row-info-tile__metadata" { span { (metadata) } }
                    div class="ui-row-info-tile__badges" { @for item in badges { (item) } }
                }
            }
        }
    }

    pub fn receipt_panel(status: &ConsoleStatus, message: &str) -> Markup {
        html! {
            section class="ui-card receipt-card" aria-live="polite" {
                div class="ui-card__header" {
                    "Receipt membrane"
                    (badge(BadgeVariant::Success, "rendered"))
                }
                div class="ui-card__body" {
                    p { (message) }
                    div class="receipt-grid" {
                        code { "source=" (status.receipts.source) }
                        code { "runtime=" (status.receipts.runtime) }
                        code { "update=" (status.receipts.update) }
                    }
                }
            }
        }
    }
}

mod anyhow_free {
    pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
}
