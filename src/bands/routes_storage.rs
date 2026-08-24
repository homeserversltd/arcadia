async fn index(State(state): State<Arc<AppState>>) -> maud::Markup {
    ui::layout(&state.living_snapshot().status)
}

async fn health(State(state): State<Arc<AppState>>) -> Json<Health> {
    Json(Health {
        ok: true,
        service: "arcadia",
        product: state.product.clone(),
        version: env!("CARGO_PKG_VERSION"),
        build_sha: option_env!("ARCADIA_BUILD_SHA"),
        started_unix: state.started_unix,
    })
}

async fn status(State(state): State<Arc<AppState>>) -> Json<ConsoleStatus> {
    Json(state.living_snapshot().status.clone())
}

async fn storage_state_route(State(state): State<Arc<AppState>>) -> Json<StorageStatus> {
    Json(state.living_snapshot().storage.clone())
}

async fn storage_summary_route(State(state): State<Arc<AppState>>) -> Json<StorageStatus> {
    Json(state.living_snapshot().storage.clone())
}

async fn storage_registry_route(State(state): State<Arc<AppState>>) -> Json<StorageRegistry> {
    Json(state.living_snapshot().storage.registry.clone())
}

async fn storage_rescan_route(State(state): State<Arc<AppState>>) -> Json<StorageStatus> {
    state.request_living_refresh();
    Json(state.living_snapshot().storage.clone())
}

async fn storage_category_route(
    State(state): State<Arc<AppState>>,
    AxumPath(category): AxumPath<String>,
) -> Json<serde_json::Value> {
    let storage = state.living_snapshot().storage.clone();
    let value = match category.as_str() {
        "games" => {
            serde_json::json!({"category":"games","summary":storage.categories.games,"items":storage.game_folders})
        }
        "artwork" => {
            serde_json::json!({"category":"artwork","summary":storage.categories.artwork,"items":storage.artwork_stores})
        }
        "ai-models" | "ai" => {
            serde_json::json!({"category":"ai-models","summary":storage.categories.ai_models,"items":storage.ai_model_files,"roots":storage.registry.categories.ai_models.roots})
        }
        "updates" => {
            serde_json::json!({"category":"updates","summary":storage.categories.updates,"roots":storage.registry.categories.updates.roots})
        }
        "logs" => {
            serde_json::json!({"category":"logs","summary":storage.categories.logs,"roots":storage.registry.categories.logs.roots})
        }
        "temporary" | "temp" => {
            serde_json::json!({"category":"temporary","summary":storage.categories.temporary,"roots":storage.registry.categories.temporary.roots})
        }
        "system" => {
            serde_json::json!({"category":"system","summary":storage.categories.system,"roots":storage.registry.categories.system.roots})
        }
        "other" => serde_json::json!({"category":"other","summary":storage.categories.other}),
        "free" => {
            serde_json::json!({"category":"free","bytes":storage.free_bytes,"size":storage.free,"percentOfTotal":100u8.saturating_sub(storage.percent_used)})
        }
        _ => serde_json::json!({"category":category,"error":"Unknown storage category"}),
    };
    Json(value)
}

async fn storage_rescan_folder_route(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RescanFolderRequest>,
) -> (StatusCode, Json<FolderStorage>) {
    let empty_folder = |id: &str, display_name: &str, path: &str, state: &str| FolderStorage {
        id: id.to_string(),
        display_name: display_name.to_string(),
        path: path.to_string(),
        bytes: 0,
        size: "0 B".to_string(),
        file_count: 0,
        last_modified_at: None,
        state: state.to_string(),
    };
    if !is_managed_storage_path(&body.path) {
        return (
            StatusCode::BAD_REQUEST,
            Json(empty_folder("unmanaged", "Unmanaged", &body.path, "unknown")),
        );
    }
    let storage = state.living_snapshot().storage.clone();
    let folder = storage
        .game_folders
        .iter()
        .find(|folder| folder.path == body.path)
        .map(|folder| FolderStorage {
            id: folder.platform.clone(),
            display_name: folder.display_name.clone(),
            path: folder.path.clone(),
            bytes: folder.bytes,
            size: folder.size.clone(),
            file_count: folder.file_count,
            last_modified_at: None,
            state: "ok".to_string(),
        })
        .or_else(|| {
            storage
                .artwork_stores
                .iter()
                .find(|folder| folder.path == body.path)
                .map(|folder| folder.clone())
        })
        .unwrap_or_else(|| empty_folder("queued", "Refresh queued", &body.path, "unknown"));
    state.request_living_refresh();
    (StatusCode::OK, Json(folder))
}

async fn storage_game_folders_route(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<GameFolderStorage>> {
    Json(state.living_snapshot().storage.game_folders.clone())
}

async fn storage_games_route(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let storage = state.living_snapshot().storage.clone();
    let library = state.living_snapshot().status.library.clone();
    Json(serde_json::json!({
        "summary": storage.categories.games,
        "folders": storage.game_folders,
        "gamescopeProfiles": library.gamescope_profiles,
        "gamescopeInstalledGames": library.gamescope_installed_games,
        "syncStatusAvailable": library.last_sync_state != "unknown",
        "syncUnavailableMessage": "Sync status unavailable for game folders."
    }))
}

async fn storage_gamescope_route(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let inventory = state.living_snapshot().status.library.clone();
    Json(serde_json::json!({
        "profiles": inventory.gamescope_profiles,
        "installedGames": inventory.gamescope_installed_games,
    }))
}

async fn storage_game_platform_route(
    State(state): State<Arc<AppState>>,
    AxumPath(platform): AxumPath<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let snapshot = state.living_snapshot();
    let wanted = platform.to_ascii_lowercase();
    let Some(folder) = snapshot
        .storage
        .game_folders
        .iter()
        .find(|folder| folder.platform == wanted)
    else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"ok":false,"message":"Unknown game platform"})),
        );
    };
    (
        StatusCode::OK,
        Json(serde_json::json!({"ok":true,"folder":folder})),
    )
}

async fn storage_artwork_route(State(state): State<Arc<AppState>>) -> Json<Vec<FolderStorage>> {
    Json(state.living_snapshot().storage.artwork_stores.clone())
}

async fn storage_ai_models_route(State(state): State<Arc<AppState>>) -> Json<Vec<AIModelStorage>> {
    Json(state.living_snapshot().storage.ai_model_files.clone())
}

async fn storage_cleanup_route(State(state): State<Arc<AppState>>) -> Json<CleanupState> {
    Json(state.living_snapshot().storage.cleanup.clone())
}

async fn storage_locations_route(State(state): State<Arc<AppState>>) -> Json<StorageRegistry> {
    Json(state.living_snapshot().storage.registry.clone())
}

async fn storage_diagnostics_route(State(state): State<Arc<AppState>>) -> Json<StorageDiagnostics> {
    Json(state.living_snapshot().storage.diagnostics.clone())
}

async fn storage_cleanup_artwork_route(
    Json(body): Json<ConsoleActionRequest>,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    action_clear_artwork_cache(Json(body)).await
}

async fn storage_cleanup_temporary_route(
    Json(body): Json<ConsoleActionRequest>,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    action_clean_temporary_files(Json(body)).await
}

async fn storage_cleanup_partial_ai_downloads_route(
    Json(body): Json<ConsoleActionRequest>,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    if body.confirm.as_deref() != Some("CLEAR_PARTIAL_DOWNLOADS") {
        return console_action_error(
            StatusCode::BAD_REQUEST,
            "clear-partial-ai-downloads",
            "arcadia-storage",
            "Confirm before clearing partial AI downloads.",
        );
    }
    cleanup_known_roots(
        "clear-partial-ai-downloads",
        partial_ai_download_roots(),
        "Partial AI downloads cleared. Installed models were not removed.",
    )
}

async fn storage_cleanup_old_updates_route(
    Json(body): Json<ConsoleActionRequest>,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    if body.confirm.as_deref() != Some("CLEAR_OLD_UPDATES") {
        return console_action_error(
            StatusCode::BAD_REQUEST,
            "clear-old-updates",
            "arcadia-storage",
            "Confirm before clearing old update packages.",
        );
    }
    cleanup_known_roots(
        "clear-old-updates",
        update_roots(),
        "Old update packages cleared.",
    )
}

async fn storage_cleanup_logs_route(
    Json(body): Json<ConsoleActionRequest>,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    if body.confirm.as_deref() != Some("PRUNE_LOGS") {
        return console_action_error(
            StatusCode::BAD_REQUEST,
            "prune-logs",
            "arcadia-storage",
            "Confirm before pruning managed logs.",
        );
    }
    cleanup_known_roots("prune-logs", log_roots(), "Managed logs pruned.")
}

async fn storage_create_managed_folder_route(
    Json(body): Json<CreateManagedFolderRequest>,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    if !is_managed_storage_path(&body.path) {
        return console_action_error(
            StatusCode::BAD_REQUEST,
            "create-managed-folder",
            "arcadia-storage",
            "Only configured managed storage roots can be created.",
        );
    }
    match fs::create_dir_all(&body.path) {
        Ok(()) => (
            StatusCode::OK,
            Json(ConsoleActionResponse {
                ok: true,
                action: "create-managed-folder",
                command: "arcadia-storage",
                exit_code: Some(0),
                message: format!("Managed folder created: {}", body.path),
                stdout: String::new(),
                stderr: String::new(),
            }),
        ),
        Err(err) => console_action_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "create-managed-folder",
            "arcadia-storage",
            &format!("Managed folder could not be created: {err}"),
        ),
    }
}



async fn storage_summary_rescan_route(State(state): State<Arc<AppState>>) -> Json<StorageStatus> {
    state.request_living_refresh();
    Json(state.living_snapshot().storage.clone())
}


async fn storage_category_rescan_route(
    State(state): State<Arc<AppState>>,
    AxumPath(category): AxumPath<String>,
) -> Json<serde_json::Value> {
    state.request_living_refresh();
    storage_category_route(State(state), AxumPath(category)).await
}


async fn storage_game_platform_rescan_route(
    State(state): State<Arc<AppState>>,
    AxumPath(platform): AxumPath<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    state.request_living_refresh();
    storage_game_platform_route(State(state), AxumPath(platform)).await
}
