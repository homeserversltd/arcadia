async fn index(State(state): State<Arc<AppState>>) -> maud::Markup {
    ui::layout(&console_status(&state))
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
    Json(console_status(&state))
}

async fn storage_state_route() -> Json<StorageStatus> {
    Json(storage_status())
}

async fn storage_summary_route() -> Json<StorageStatus> {
    Json(storage_status())
}

async fn storage_registry_route() -> Json<StorageRegistry> {
    Json(storage_registry(&network_status()))
}

async fn storage_rescan_route() -> Json<StorageStatus> {
    Json(storage_status())
}

async fn storage_category_route(AxumPath(category): AxumPath<String>) -> Json<serde_json::Value> {
    let storage = storage_status();
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
    Json(body): Json<RescanFolderRequest>,
) -> (StatusCode, Json<FolderStorage>) {
    if !is_managed_storage_path(&body.path) {
        return (
            StatusCode::BAD_REQUEST,
            Json(folder_storage("unmanaged", "Unmanaged", &body.path)),
        );
    }
    (
        StatusCode::OK,
        Json(folder_storage("managed", "Managed folder", &body.path)),
    )
}

async fn storage_game_folders_route() -> Json<Vec<GameFolderStorage>> {
    Json(storage_scan().game_folders)
}

async fn storage_games_route() -> Json<serde_json::Value> {
    let storage = storage_scan();
    let library = library_status(&storage);
    Json(serde_json::json!({
        "summary": storage.categories.games,
        "folders": storage.game_folders,
        "gamescopeProfiles": library.gamescope_profiles,
        "gamescopeInstalledGames": library.gamescope_installed_games,
        "syncStatusAvailable": load_sync_manifest().is_some(),
        "syncUnavailableMessage": "Sync status unavailable for game folders."
    }))
}

async fn storage_gamescope_route() -> Json<serde_json::Value> {
    let inventory = gamescope_inventory();
    Json(serde_json::json!({
        "profiles": inventory.profiles,
        "installedGames": inventory.entries,
    }))
}

async fn storage_game_platform_route(
    AxumPath(platform): AxumPath<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let registry = storage_registry(&network_status());
    let wanted = platform.to_ascii_lowercase();
    let Some(root) = registry
        .categories
        .games
        .roots
        .iter()
        .find(|r| r.id == wanted)
    else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"ok":false,"message":"Unknown game platform"})),
        );
    };
    let folder = game_folder_storage(root);
    (
        StatusCode::OK,
        Json(serde_json::json!({"ok":true,"folder":folder})),
    )
}

async fn storage_artwork_route() -> Json<Vec<FolderStorage>> {
    Json(storage_scan().artwork_stores)
}

async fn storage_ai_models_route() -> Json<Vec<AIModelStorage>> {
    Json(storage_scan().ai_model_files)
}

async fn storage_cleanup_route() -> Json<CleanupState> {
    Json(storage_scan().cleanup)
}

async fn storage_locations_route() -> Json<StorageRegistry> {
    Json(storage_registry(&network_status()))
}

async fn storage_diagnostics_route() -> Json<StorageDiagnostics> {
    Json(storage_scan().diagnostics)
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

