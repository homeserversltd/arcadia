fn storage_status() -> StorageStatus {
    storage_scan()
}

fn storage_scan() -> StorageStatus {
    let scan_started = SystemTime::now();
    let network = network_status();
    let registry = storage_registry(&network);
    let volume = root_volume();
    let total_bytes = volume.total_bytes;
    let used_bytes = volume.used_bytes;
    let free_bytes = volume.free_bytes;
    let percent_used = percent(used_bytes, total_bytes);
    let (health, header_state, header_class) = storage_health(percent_used);

    let game_folders = registry
        .categories
        .games
        .roots
        .iter()
        .map(game_folder_storage)
        .collect::<Vec<_>>();
    let games_bytes = game_folders.iter().map(|f| f.bytes).sum::<u64>();
    let games_files = game_folders.iter().map(|f| f.file_count).sum::<u64>();

    let artwork_stores = registry
        .categories
        .artwork
        .roots
        .iter()
        .map(|r| folder_storage(&r.id, &r.display_name, &r.path))
        .collect::<Vec<_>>();
    let artwork_bytes = artwork_stores.iter().map(|f| f.bytes).sum::<u64>();
    let artwork_files = artwork_stores.iter().map(|f| f.file_count).sum::<u64>();

    let ai_model_files = ai_model_files();
    let ai_model_bytes = ai_model_files.iter().map(|m| m.bytes).sum::<u64>();

    let update_stores = registry
        .categories
        .updates
        .roots
        .iter()
        .map(|r| folder_storage(&r.id, &r.display_name, &r.path))
        .collect::<Vec<_>>();
    let updates_bytes = update_stores.iter().map(|f| f.bytes).sum::<u64>();
    let updates_files = update_stores.iter().map(|f| f.file_count).sum::<u64>();

    let log_stores = registry
        .categories
        .logs
        .roots
        .iter()
        .map(|r| folder_storage(&r.id, &r.display_name, &r.path))
        .collect::<Vec<_>>();
    let logs_bytes = log_stores.iter().map(|f| f.bytes).sum::<u64>();
    let logs_files = log_stores.iter().map(|f| f.file_count).sum::<u64>();

    let temp_stores = registry
        .categories
        .temporary
        .roots
        .iter()
        .map(|r| folder_storage(&r.id, &r.display_name, &r.path))
        .collect::<Vec<_>>();
    let temporary_bytes = temp_stores.iter().map(|f| f.bytes).sum::<u64>();
    let temporary_files = temp_stores.iter().map(|f| f.file_count).sum::<u64>();

    let system_stores = registry
        .categories
        .system
        .roots
        .iter()
        .map(|r| folder_storage(&r.id, &r.display_name, &r.path))
        .collect::<Vec<_>>();
    let system_bytes = system_stores.iter().map(|f| f.bytes).sum::<u64>();
    let system_files = system_stores.iter().map(|f| f.file_count).sum::<u64>();

    let classified = games_bytes
        .saturating_add(artwork_bytes)
        .saturating_add(ai_model_bytes)
        .saturating_add(updates_bytes)
        .saturating_add(logs_bytes)
        .saturating_add(temporary_bytes)
        .saturating_add(system_bytes);
    let other_bytes = used_bytes.saturating_sub(classified);

    let games = category_status(
        games_bytes,
        games_files,
        registry.categories.games.roots.len(),
        used_bytes,
        total_bytes,
        "game files",
        if game_folders.iter().any(|f| f.path_missing()) {
            "warning"
        } else {
            "ok"
        },
        "Per-platform Samba game folders.",
    );
    let artwork = category_status(
        artwork_bytes,
        artwork_files,
        registry.categories.artwork.roots.len(),
        used_bytes,
        total_bytes,
        "artwork files",
        state_for_roots(&artwork_stores),
        "Covers, metadata, generated artwork, and scraper cache.",
    );
    let ai_models_cat = category_status(
        ai_model_bytes,
        ai_model_files.len() as u64,
        registry.categories.ai_models.roots.len(),
        used_bytes,
        total_bytes,
        "model files",
        "ok",
        "Installed Local AI model files and download state.",
    );
    let updates = category_status(
        updates_bytes,
        updates_files,
        registry.categories.updates.roots.len(),
        used_bytes,
        total_bytes,
        "update files",
        state_for_roots(&update_stores),
        "Managed update package/cache roots.",
    );
    let logs = category_status(
        logs_bytes,
        logs_files,
        registry.categories.logs.roots.len(),
        used_bytes,
        total_bytes,
        "log files",
        state_for_roots(&log_stores),
        "Managed log roots only.",
    );
    let temporary = category_status(
        temporary_bytes,
        temporary_files,
        registry.categories.temporary.roots.len(),
        used_bytes,
        total_bytes,
        "temporary files",
        state_for_roots(&temp_stores),
        "Safe temporary roots only.",
    );
    let system = category_status(
        system_bytes,
        system_files,
        registry.categories.system.roots.len(),
        used_bytes,
        total_bytes,
        "system files",
        state_for_roots(&system_stores),
        "Configured system/runtime roots.",
    );
    let other = category_status(
        other_bytes,
        0,
        0,
        used_bytes,
        total_bytes,
        "unclassified",
        "unknown",
        "Used space not classified by the managed storage registry.",
    );

    let ai_rows = ai_model_files
        .iter()
        .map(|m| AiModelDiskStatus {
            friendly_name: m.name.clone(),
            filename: m.filename.clone(),
            size: m.size.clone(),
            status: if m.loaded { "Hot" } else { "Installed" },
        })
        .collect::<Vec<_>>();
    let ai_models = AiModelStorageStatus {
        bytes: ai_model_bytes,
        size: human_size(ai_model_bytes),
        count: ai_model_files.len(),
        meta: format!("{} installed models", ai_model_files.len()),
        detail: if ai_model_files.is_empty() {
            "No local AI model files were found in the Local AI registry roots.".to_string()
        } else {
            "Installed model files are not cache. Remove only unused cold models.".to_string()
        },
        percent_of_total: percent(ai_model_bytes, total_bytes),
        models: ai_rows,
    };

    let diagnostics = storage_diagnostics(
        &registry,
        &game_folders,
        games_bytes,
        scan_started
            .elapsed()
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
        &volume,
    );
    let warning_copy = match percent_used {
        98..=100 => "Storage full. Free space before syncing or downloading models.",
        90..=97 => "Storage low. Sync, updates, and model downloads may fail.",
        75..=89 => "Storage is getting full. Review cleanup opportunities before large downloads.",
        _ => "Storage has enough free space for appliance work.",
    };
    StorageStatus {
        scanned_at: now_rfc3339_like(),
        scanning: false,
        scan_error: None,
        total_bytes,
        used_bytes,
        free_bytes,
        percent_used,
        thresholds: StorageThresholds {
            getting_full_percent: 75,
            low_percent: 90,
            full_percent: 98,
        },
        volumes: vec![volume.clone()],
        registry,
        categories: StorageCategories {
            games: games.clone(),
            artwork: artwork.clone(),
            ai_models: ai_models_cat.clone(),
            updates: updates.clone(),
            logs: logs.clone(),
            temporary: temporary.clone(),
            system: system.clone(),
            other: other.clone(),
        },
        game_folders,
        artwork_stores,
        ai_model_files,
        cleanup: CleanupState {
            artwork_bytes_clearable: artwork_bytes,
            temporary_bytes_clearable: temporary_bytes,
            partial_downloads_bytes_clearable: partial_ai_download_roots()
                .iter()
                .map(|p| path_usage(Path::new(p)).bytes)
                .sum(),
            old_update_bytes_clearable: updates_bytes,
            logs_bytes_clearable: logs_bytes,
        },
        diagnostics,
        health,
        header_state,
        header_class,
        header_tooltip: format!(
            "Storage {}: {} free · {}% used",
            health,
            human_size(free_bytes),
            percent_used
        ),
        ok_copy: "Storage has enough free space for appliance work.",
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

impl GameFolderStorage {
    fn path_missing(&self) -> bool {
        !Path::new(&self.path).exists()
    }
}

