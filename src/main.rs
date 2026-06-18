use axum::{
    extract::State,
    http::{header, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::Serialize;
use std::{
    env,
    net::SocketAddr,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

const INDEX_HTML: &str = include_str!("../static/index.html");
const APP_CSS: &str = include_str!("../static/app.css");
const APP_JS: &str = include_str!("../static/app.js");

#[derive(Clone)]
struct AppState {
    started_unix: u64,
    canonical_url: String,
    product: String,
}

#[derive(Serialize)]
struct Health<'a> {
    ok: bool,
    service: &'a str,
    product: &'a str,
    version: &'a str,
    started_unix: u64,
}

#[derive(Serialize)]
struct ConsoleStatus<'a> {
    schema: &'a str,
    product: &'a str,
    canonical_url: &'a str,
    arcadia: ArcadiaStatus<'a>,
    vault: VaultStatus<'a>,
    surfaces: SurfaceStatus<'a>,
}

#[derive(Serialize)]
struct ArcadiaStatus<'a> {
    service: &'a str,
    version: &'a str,
    mode: &'a str,
}

#[derive(Serialize)]
struct VaultStatus<'a> {
    policy: &'a str,
    state_path: &'a str,
    unlock_surface: &'a str,
}

#[derive(Serialize)]
struct SurfaceStatus<'a> {
    http: &'a str,
    mdns: &'a str,
    smb: &'a str,
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

async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn health(State(state): State<Arc<AppState>>) -> Json<Health<'static>> {
    Json(Health {
        ok: true,
        service: "arcadia",
        product: Box::leak(state.product.clone().into_boxed_str()),
        version: env!("CARGO_PKG_VERSION"),
        started_unix: state.started_unix,
    })
}

async fn status(State(state): State<Arc<AppState>>) -> Json<ConsoleStatus<'static>> {
    Json(ConsoleStatus {
        schema: "arcadia.status.v1",
        product: Box::leak(state.product.clone().into_boxed_str()),
        canonical_url: Box::leak(state.canonical_url.clone().into_boxed_str()),
        arcadia: ArcadiaStatus {
            service: "arcadia",
            version: env!("CARGO_PKG_VERSION"),
            mode: "scaffold",
        },
        vault: VaultStatus {
            policy: "manual-unlock-default",
            state_path: "/var/lib/homeconsole/state.json",
            unlock_surface: "pending-gui-action",
        },
        surfaces: SurfaceStatus {
            http: "0.0.0.0:8080 behind LAN port 80 redirect",
            mdns: "homeconsole.local",
            smb: "HOMECONSOLE",
        },
    })
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

mod anyhow_free {
    pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
}
