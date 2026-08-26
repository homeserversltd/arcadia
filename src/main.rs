use axum::{
    extract::{DefaultBodyLimit, Multipart, Path as AxumPath, Query, Request, State},
    http::{header, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    routing::{get, post},
    Json, Router,
};
use futures_core::Stream;
use serde::{Deserialize, Serialize};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    convert::Infallible,
    env, fs,
    fs::OpenOptions,
    io::Write,
    net::{Ipv4Addr, TcpStream},
    path::{Path, PathBuf},
    process::Command,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, OnceLock,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::{Event as TracingEvent, Id, Level, Subscriber};
use tracing_subscriber::{
    layer::{Context as TracingContext, SubscriberExt},
    registry::LookupSpan,
    util::SubscriberInitExt,
    Layer,
};

mod serving;
mod ui;

// Arcadia infinite-infinite bands: keep main.rs as the thin process face,
// while each included band preserves one coherent transition surface.
include!("bands/constants.rs");
include!("bands/state_core.rs");
include!("bands/state_surface_network.rs");
include!("bands/state_library_ai.rs");
include!("bands/state_actions.rs");
include!("bands/routes_storage.rs");
include!("bands/routes_ai_models.rs");
include!("bands/routes_ai_runtime.rs");
include!("bands/routes_ai_settings.rs");
include!("bands/routes_caduceus.rs");
include!("bands/caduceus_access.rs");
include!("bands/console_system_actions.rs");
include!("bands/console_access_actions.rs");
include!("bands/provider_keys.rs");
include!("bands/network_wifi.rs");
include!("bands/network_diagnostics_pin.rs");
include!("bands/assets.rs");
include!("bands/status_core.rs");
include!("bands/status_network.rs");
include!("bands/status_system.rs");
include!("bands/status_library.rs");
include!("bands/status_controllers.rs");
include!("bands/benchmark.rs");
include!("bands/local_ai_config.rs");
include!("bands/local_ai_runtime.rs");
include!("bands/surface.rs");
include!("bands/storage_scan.rs");
include!("bands/storage_registry.rs");
include!("bands/storage_diagnostics.rs");
include!("bands/storage_cleanup.rs");
include!("bands/api_root.rs");
include!("bands/anyhow_free.rs");

const ARCADIA_HYALOS_MESSAGE_MAX_CHARS: usize = 512;

#[derive(Default)]
struct ArcadiaHyalosFields {
    correlation_id: Option<String>,
    message: Option<String>,
    kind: Option<String>,
    result: Option<String>,
    ok: Option<bool>,
    attributes: BTreeMap<String, serde_json::Value>,
}

impl ArcadiaHyalosFields {
    fn record_text(&mut self, field: &tracing::field::Field, value: String) {
        match field.name() {
            "correlation_id" => self.correlation_id = Some(value),
            "message" => self.message = Some(value),
            "kind" => self.kind = Some(value),
            "result" => self.result = Some(value),
            name if name != "ok" && self.attributes.len() < 32 => {
                self.attributes
                    .insert(name.to_string(), serde_json::Value::String(value));
            }
            _ => {}
        }
    }
}

impl tracing::field::Visit for ArcadiaHyalosFields {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.record_text(field, format!("{value:?}"));
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.record_text(field, value.to_string());
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        if field.name() == "ok" {
            self.ok = Some(value);
        } else if self.attributes.len() < 32 {
            self.attributes
                .insert(field.name().to_string(), serde_json::Value::Bool(value));
        }
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        if self.attributes.len() < 32 {
            self.attributes
                .insert(field.name().to_string(), serde_json::json!(value));
        }
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        if self.attributes.len() < 32 {
            self.attributes
                .insert(field.name().to_string(), serde_json::json!(value));
        }
    }

    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        if self.attributes.len() < 32 {
            self.attributes
                .insert(field.name().to_string(), serde_json::json!(value));
        }
    }
}

struct ArcadiaHyalosLayer;

impl ArcadiaHyalosLayer {
    fn forwards(level: &Level) -> bool {
        matches!(*level, Level::ERROR | Level::WARN | Level::INFO)
    }

    fn reflect(
        kind: &'static str,
        level: &Level,
        target: &str,
        name: &str,
        mut fields: ArcadiaHyalosFields,
    ) {
        if !Self::forwards(level) {
            return;
        }
        let message = fields
            .message
            .take()
            .unwrap_or_else(|| format!("{kind}: {name}"));
        let kind = fields.kind.take().unwrap_or_else(|| kind.to_string());
        let mut attributes = fields.attributes;
        attributes.insert(
            "target".to_string(),
            serde_json::Value::String(target.to_string()),
        );
        attributes.insert(
            "name".to_string(),
            serde_json::Value::String(name.to_string()),
        );
        let reflection = serde_json::json!({
            "organ": "arcadia",
            "kind": kind,
            "level": level.as_str().to_ascii_lowercase(),
            "message": message.chars().take(ARCADIA_HYALOS_MESSAGE_MAX_CHARS).collect::<String>(),
            "world": "backend",
            "correlation_id": fields.correlation_id,
            "ok": fields.ok,
            "result": fields.result,
            "attributes_redacted": attributes,
        });
        let Ok(body) = serde_json::to_string(&reflection) else {
            return;
        };
        let _ = std::thread::Builder::new()
            .name("arcadia-hyalos".to_string())
            .spawn(move || {
                let _ = caduceus_post_json_with_timeout("/api/v1/log/reflect", &body, "2");
            });
    }
}

impl<S> Layer<S> for ArcadiaHyalosLayer
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn on_event(&self, event: &TracingEvent<'_>, _context: TracingContext<'_, S>) {
        let metadata = event.metadata();
        let mut fields = ArcadiaHyalosFields::default();
        event.record(&mut fields);
        Self::reflect(
            "tracing-event",
            metadata.level(),
            metadata.target(),
            metadata.name(),
            fields,
        );
    }

    fn on_new_span(
        &self,
        attributes: &tracing::span::Attributes<'_>,
        _id: &Id,
        _context: TracingContext<'_, S>,
    ) {
        let metadata = attributes.metadata();
        let mut fields = ArcadiaHyalosFields::default();
        attributes.record(&mut fields);
        Self::reflect(
            "tracing-span",
            metadata.level(),
            metadata.target(),
            metadata.name(),
            fields,
        );
    }

    fn on_record(
        &self,
        id: &Id,
        values: &tracing::span::Record<'_>,
        context: TracingContext<'_, S>,
    ) {
        let Some(span) = context.span(id) else {
            return;
        };
        let metadata = span.metadata();
        let mut fields = ArcadiaHyalosFields::default();
        values.record(&mut fields);
        Self::reflect(
            "tracing-span",
            metadata.level(),
            metadata.target(),
            metadata.name(),
            fields,
        );
    }
}

async fn living_readiness(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    // Liveness is deliberately available before the first deferred refresh; every
    // other route is state-backed and must wait for a coherent living snapshot.
    if request.uri().path() == "/health" || state.living.is_initialized() {
        return next.run(request).await;
    }

    (
        StatusCode::SERVICE_UNAVAILABLE,
        [(header::RETRY_AFTER, HeaderValue::from_static("1"))],
        Json(serde_json::json!({
            "ok": false,
            "error": "living state is initializing",
            "retryable": true,
        })),
    )
        .into_response()
}

#[tokio::main]
async fn main() -> anyhow_free::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .finish()
        .with(ArcadiaHyalosLayer)
        .init();

    let args = env::args().skip(1).collect::<Vec<_>>();
    let serve_config = serving::ServeConfig::from_env_and_args(&args)?;
    let started_unix = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let state = Arc::new(AppState {
        started_unix,
        canonical_url: env::var("ARCADIA_CANONICAL_URL").unwrap_or_else(|_| {
            if serve_config.https_bind.is_some() {
                "https://console.home.arpa/".to_string()
            } else {
                "http://console.home.arpa/".to_string()
            }
        }),
        product: "HomeConsole".to_string(),
        living: Arc::new(ArcadiaLivingMachine::new()),
    });
    let app = Router::new()
        .route("/", get(index))
        .route("/health", get(health))
        .route("/api", get(api_root_route))
        .route("/api/root", get(api_root_route))
        .route("/api/root/events", get(api_root_events_route))
        .route("/api/root/events/renew", post(api_root_events_renew_route))
        .route("/api/status", get(status))
        .route("/api/benchmark/status", get(benchmark_status_route))
        .route("/api/benchmark/compose", post(benchmark_compose_route))
        .route("/api/benchmark/uncompose", post(benchmark_uncompose_route))
        .route("/api/benchmark/sample", post(benchmark_sample_route))
        .route("/api/storage/state", get(storage_state_route))
        .route("/api/storage/summary", get(storage_summary_route))
        .route(
            "/api/storage/rescan-summary",
            post(storage_summary_rescan_route),
        )
        .route("/api/storage/registry", get(storage_registry_route))
        .route("/api/storage/rescan", post(storage_rescan_route))
        .route(
            "/api/storage/category/:category",
            get(storage_category_route),
        )
        .route(
            "/api/storage/category/:category/rescan",
            post(storage_category_rescan_route),
        )
        .route(
            "/api/storage/rescan-folder",
            post(storage_rescan_folder_route),
        )
        .route(
            "/api/storage/cleanup/artwork",
            post(storage_cleanup_artwork_route),
        )
        .route(
            "/api/storage/cleanup/temporary",
            post(storage_cleanup_temporary_route),
        )
        .route(
            "/api/storage/cleanup/partial-ai-downloads",
            post(storage_cleanup_partial_ai_downloads_route),
        )
        .route(
            "/api/storage/cleanup/old-updates",
            post(storage_cleanup_old_updates_route),
        )
        .route(
            "/api/storage/cleanup/logs",
            post(storage_cleanup_logs_route),
        )
        .route(
            "/api/storage/create-managed-folder",
            post(storage_create_managed_folder_route),
        )
        .route("/api/storage/game-folders", get(storage_game_folders_route))
        .route("/api/storage/games", get(storage_games_route))
        .route("/api/storage/gamescope", get(storage_gamescope_route))
        .route(
            "/api/storage/games/:platform",
            get(storage_game_platform_route),
        )
        .route(
            "/api/storage/games/:platform/rescan",
            post(storage_game_platform_rescan_route),
        )
        .route("/api/storage/artwork", get(storage_artwork_route))
        .route("/api/storage/ai-models", get(storage_ai_models_route))
        .route("/api/storage/cleanup", get(storage_cleanup_route))
        .route("/api/storage/locations", get(storage_locations_route))
        .route("/api/storage/diagnostics", get(storage_diagnostics_route))
        .route("/api/network/state", get(network_state_route))
        .route("/api/ai/state", get(ai_state_route))
        .route("/api/controllers/state", get(controllers_state_route))
        .route("/api/controllers/input", get(controllers_input_route))
        .route(
            "/api/controllers/trainer/events",
            get(controllers_trainer_events_route),
        )
        .route("/api/network/wifi/status", get(wifi_status))
        .route("/api/network/wifi/scan", post(wifi_scan))
        .route("/api/network/wifi/connect", post(wifi_connect))
        .route("/api/network/wifi/disconnect", post(wifi_disconnect))
        .route("/api/network/wifi/forget", post(wifi_forget))
        .route("/api/network/wifi/set-enabled", post(wifi_set_enabled))
        .route(
            "/api/network/ethernet/renew-dhcp",
            post(ethernet_renew_dhcp),
        )
        .route("/api/network/ip/apply", post(ip_apply))
        .route("/api/network/ip/confirm", post(ip_confirm))
        .route("/api/network/ip/rollback", post(ip_rollback))
        .route("/api/network/diagnostics/run", post(diagnostics_run))
        .route("/api/network/speed-test", post(speed_test_run))
        .route("/api/system/status", get(system_status_route))
        .route("/api/system/ssh/service", post(action_ssh_service))
        .route(
            "/api/system/ssh/password-auth",
            post(action_ssh_password_auth),
        )
        .route(
            "/api/system/ssh/authorized-key",
            post(action_install_authorized_key),
        )
        .route("/api/system/trust/root-ca", post(action_install_root_ca))
        .route("/api/system/trust/mode", post(action_set_trust_mode))
        .route("/api/actions/restart-arcadia", post(action_restart_arcadia))
        .route(
            "/api/v1/admin-admittance/open",
            post(caduceus_attendance_open_route).layer(DefaultBodyLimit::max(4 * 1024)),
        )
        .route(
            "/api/v1/admin-admittance/validate",
            post(caduceus_attendance_validate_route),
        )
        .route(
            "/api/v1/admin-admittance/invalidate",
            post(caduceus_attendance_invalidate_route),
        )
        .route("/api/gui-pin/status", get(gui_pin_status_route))
        .route("/api/vault/status", get(caduceus_vault_status_proxy_route))
        .route("/api/vault/unlock", post(caduceus_vault_unlock_proxy_route))
        .route(
            "/api/vault/auto-decrypt",
            post(caduceus_vault_auto_decrypt_proxy_route),
        )
        .route("/api/gui-pin/access", post(caduceus_pin_access_route))
        .route("/api/gui-pin/change", post(caduceus_pin_change_route))
        .route("/api/provider-keys/status", get(provider_keys_status))
        .route("/api/provider-keys/save", post(save_provider_keys))
        .route(
            "/api/ai/runtime/check-update",
            post(ai_runtime_check_update),
        )
        .route("/api/ai/runtime/update", post(ai_runtime_update))
        .route("/api/ai/runtime/restart", post(ai_runtime_restart))
        .route("/api/ai/models/installed", get(ai_models_installed))
        .route("/api/ai/models/recommended", get(ai_models_recommended))
        .route(
            "/api/ai/models/install-recommended",
            post(ai_install_recommended),
        )
        .route(
            "/api/ai/models/huggingface/list-files",
            post(ai_hf_list_files),
        )
        .route("/api/ai/models/huggingface/download", post(ai_hf_download))
        .route("/api/ai/models/import", post(ai_model_import))
        .route("/api/ai/models/rescan", post(ai_models_rescan))
        .route("/api/ai/models/download/cancel", post(ai_download_cancel))
        .route("/api/ai/models/remove", post(ai_model_remove))
        .route("/api/ai/model/select", post(ai_model_select))
        .route("/api/ai/model/load", post(ai_model_load))
        .route("/api/ai/model/unload", post(ai_model_unload))
        .route(
            "/api/ai/inference/set-enabled",
            post(ai_inference_set_enabled),
        )
        .route(
            "/api/ai/inference/set-lan-access",
            post(ai_inference_set_lan_access),
        )
        .route("/api/ai/inference/test", post(ai_inference_test))
        .route("/api/ai/settings", post(ai_settings_save))
        .route("/api/actions/update-gui", post(action_update_gui))
        .route("/api/actions/check-updates", post(action_check_updates))
        .route(
            "/api/debug/emit",
            post(arcadia_debug_emit_route).layer(DefaultBodyLimit::max(16 * 1024)),
        )
        .route("/api/harmonia/ledger", get(harmonia_ledger_route))
        .route("/api/caduceus/health", get(caduceus_health_proxy_route))
        .route(
            "/api/caduceus/v1/identity",
            get(caduceus_identity_proxy_route),
        )
        .route(
            "/api/caduceus/v1/profile",
            get(caduceus_profile_proxy_route),
        )
        .route(
            "/api/caduceus/v1/health",
            get(caduceus_health_api_proxy_route),
        )
        .route(
            "/api/caduceus/v1/update/status",
            get(caduceus_update_status_proxy_route),
        )
        .route(
            "/api/caduceus/v1/cert/status",
            get(caduceus_cert_status_proxy_route),
        )
        .route(
            "/api/caduceus/v1/cert/trust-fetch",
            post(caduceus_cert_trust_fetch_proxy_route),
        )
        .route(
            "/api/caduceus/v1/cert/trust-install",
            post(caduceus_cert_trust_install_proxy_route),
        )
        .route(
            "/api/caduceus/v1/update/now",
            post(caduceus_update_now_proxy_route),
        )
        .route(
            "/api/caduceus/v1/update/check",
            post(caduceus_update_check_proxy_route),
        )
        .route(
            "/api/caduceus/v1/sync/status",
            get(caduceus_sync_status_proxy_route),
        )
        .route(
            "/api/caduceus/v1/sync/now",
            post(caduceus_sync_now_proxy_route),
        )
        .route(
            "/api/caduceus/v1/receipts/latest",
            get(caduceus_receipts_latest_proxy_route),
        )
        .route(
            "/api/caduceus/v1/receipts/ledger",
            get(caduceus_receipts_ledger_proxy_route),
        )
        .route(
            "/api/caduceus/v1/update/service/status",
            get(caduceus_update_service_status_proxy_route),
        )
        .route(
            "/api/caduceus/v1/update/service/toggle",
            post(caduceus_update_service_toggle_proxy_route),
        )
        .route(
            "/api/caduceus/v1/gui/update/now",
            post(caduceus_gui_update_now_proxy_route),
        )
        .route(
            "/api/caduceus/v1/local-ai/runtime/status",
            get(caduceus_local_ai_runtime_status_proxy_route),
        )
        .route(
            "/api/caduceus/v1/local-ai/runtime/check",
            post(caduceus_local_ai_runtime_check_proxy_route),
        )
        .route(
            "/api/caduceus/v1/local-ai/runtime/update",
            post(caduceus_local_ai_runtime_update_proxy_route),
        )
        .route(
            "/api/caduceus/v1/profile/module/toggle",
            post(caduceus_profile_module_toggle_proxy_route),
        )
        .route("/api/harmonia/module", post(action_harmonia_module_toggle))
        .route("/api/actions/sync-games", post(action_sync_games))
        .route("/api/sync/ledger", get(sync_ledger_route))
        .route(
            "/api/actions/add-games",
            post(action_add_games_upload).layer(DefaultBodyLimit::max(MAX_SYNC_UPLOAD_TOTAL_BYTES)),
        )
        .route(
            "/api/actions/controllers-rescan",
            post(action_controllers_rescan),
        )
        .route(
            "/api/actions/controllers-test",
            post(action_controllers_test),
        )
        .route(
            "/api/actions/controllers-save-profile",
            post(action_controllers_save_profile),
        )
        .route(
            "/api/actions/controllers-apply-profile",
            post(action_controllers_apply_profile),
        )
        .route(
            "/api/actions/controllers-bind",
            post(action_controllers_bind),
        )
        .route(
            "/api/actions/controllers-select",
            post(action_controllers_select),
        )
        .route(
            "/api/actions/controllers-forget",
            post(action_controllers_forget),
        )
        .route(
            "/api/actions/controllers-save-tuning",
            post(action_controllers_save_tuning),
        )
        .route(
            "/api/actions/controllers-assign-retroarch",
            post(action_controllers_assign_retroarch),
        )
        .route(
            "/api/actions/controllers-assign-dolphin",
            post(action_controllers_assign_dolphin),
        )
        .route(
            "/api/actions/controllers-assign-duckstation",
            post(action_controllers_assign_duckstation),
        )
        .route(
            "/api/actions/controllers-assign-pcsx2",
            post(action_controllers_assign_pcsx2),
        )
        .route(
            "/api/actions/controllers-assign-ppsspp",
            post(action_controllers_assign_ppsspp),
        )
        .route(
            "/api/actions/controllers-ramrod-all",
            post(action_controllers_ramrod_all),
        )
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
        .route("/api/gui-pin/reset-default", post(caduceus_pin_reset_route))
        .route("/static/app.css", get(css))
        .route("/static/indra-observation.js", get(indra_observation_js))
        .route("/static/vendor/chart.umd.min.js", get(vendor_chart_js))
        .route("/static/app.js", get(js))
        .fallback(not_found)
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            living_readiness,
        ))
        .with_state(state.clone());

    let http = match serve_config.http_bind {
        Some(addr) => {
            let listener = TcpListener::bind(addr).await?;
            tracing::info!(%addr, "arcadia HTTP listener ready");
            Some(listener)
        }
        None => None,
    };
    let https = match (
        serve_config.https_bind,
        serve_config.tls_cert.as_ref(),
        serve_config.tls_key.as_ref(),
    ) {
        (Some(addr), Some(cert), Some(key)) => {
            let tls = axum_server::tls_rustls::RustlsConfig::from_pem_file(cert, key).await?;
            tracing::info!(%addr, cert = %cert.display(), "arcadia HTTPS listener ready");
            Some((addr, tls))
        }
        (None, None, None) => None,
        _ => unreachable!("ServeConfig validates HTTPS certificate and key together"),
    };

    // Start serving before the first appliance scan so liveness probes do not
    // wait behind storage, network, or device discovery.
    let server = tokio::spawn(async move {
        match (http, https) {
            (Some(listener), Some((addr, tls))) => {
                let http_server = axum::serve(listener, app.clone());
                let https_server =
                    axum_server::bind_rustls(addr, tls).serve(app.into_make_service());
                tokio::try_join!(http_server, https_server)?;
            }
            (Some(listener), None) => axum::serve(listener, app).await?,
            (None, Some((addr, tls))) => {
                axum_server::bind_rustls(addr, tls)
                    .serve(app.into_make_service())
                    .await?;
            }
            (None, None) => return Err("no HTTP or HTTPS listener configured".into()),
        }
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    });

    let initial_state = state.clone();
    tokio::task::spawn_blocking(move || refresh_living_state(&initial_state))
        .await
        .map_err(|error| {
            std::io::Error::other(format!("initial Arcadia refresh failed: {error}"))
        })?;

    let refresh_state = state.clone();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval_at(
            tokio::time::Instant::now() + Duration::from_secs(FAST_FACTS_CADENCE_SECONDS),
            Duration::from_secs(FAST_FACTS_CADENCE_SECONDS),
        );
        loop {
            tick.tick().await;
            let state = refresh_state.clone();
            if state
                .living
                .fast_facts_refresh_due(home_telemetry_has_active_lease())
            {
                let _ = tokio::task::spawn_blocking(move || refresh_living_state(&state)).await;
            }
        }
    });

    server
        .await
        .map_err(|error| std::io::Error::other(format!("Arcadia server task failed: {error}")))??;
    Ok(())
}

include!("../tests/index.rsi");
