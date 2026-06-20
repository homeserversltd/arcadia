use axum::{
    extract::{Multipart, Path as AxumPath, Query, State},
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
    net::{Ipv4Addr, SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

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
include!("bands/local_ai_config.rs");
include!("bands/local_ai_runtime.rs");
include!("bands/surface.rs");
include!("bands/storage_scan.rs");
include!("bands/storage_registry.rs");
include!("bands/storage_diagnostics.rs");
include!("bands/storage_cleanup.rs");
include!("bands/anyhow_free.rs");

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
        canonical_url: env::var("ARCADIA_CANONICAL_URL").unwrap_or_else(|_| {
            if trust_status().mode == "https" {
                "https://console.home.arpa/".to_string()
            } else {
                "http://console.home.arpa/".to_string()
            }
        }),
        product: "HomeConsole".to_string(),
    });

    let app = Router::new()
        .route("/", get(index))
        .route("/health", get(health))
        .route("/api/status", get(status))
        .route("/api/storage/state", get(storage_state_route))
        .route("/api/storage/summary", get(storage_summary_route))
        .route("/api/storage/rescan-summary", post(storage_summary_route))
        .route("/api/storage/registry", get(storage_registry_route))
        .route("/api/storage/rescan", post(storage_rescan_route))
        .route(
            "/api/storage/category/:category",
            get(storage_category_route),
        )
        .route(
            "/api/storage/category/:category/rescan",
            post(storage_category_route),
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
        .route(
            "/api/storage/games/:platform",
            get(storage_game_platform_route),
        )
        .route(
            "/api/storage/games/:platform/rescan",
            post(storage_game_platform_route),
        )
        .route("/api/storage/artwork", get(storage_artwork_route))
        .route("/api/storage/ai-models", get(storage_ai_models_route))
        .route("/api/storage/cleanup", get(storage_cleanup_route))
        .route("/api/storage/locations", get(storage_locations_route))
        .route("/api/storage/diagnostics", get(storage_diagnostics_route))
        .route("/api/network/state", get(network_state_route))
        .route("/api/ai/state", get(ai_state_route))
        .route("/api/controllers/state", get(controllers_state_route))
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
        .route("/api/gui-pin/status", get(gui_pin_status_route))
        .route("/api/gui-pin/access", post(set_gui_pin_access))
        .route("/api/gui-pin/change", post(change_gui_pin))
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
        .route("/api/ai/token/generate", post(ai_token_generate))
        .route("/api/ai/token/revoke", post(ai_token_revoke))
        .route("/api/actions/update-gui", post(action_update_gui))
        .route("/api/actions/check-updates", post(action_check_updates))
        .route("/api/harmonia/ledger", get(harmonia_ledger_route))
        .route("/api/harmonia/module", post(action_harmonia_module_toggle))
        .route("/api/actions/sync-games", post(action_sync_games))
        .route(
            "/api/actions/controllers-rescan",
            post(action_controllers_rescan),
        )
        .route(
            "/api/actions/controllers-test",
            post(action_controllers_test),
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

include!("../tests/index.rsi");
