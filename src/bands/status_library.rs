fn library_status(storage: &StorageStatus) -> LibraryStatus {
    let game_files = current_game_files();
    let detected_games = game_files.len() as u64;
    let gamescope_entries = count_gamescope_entries();
    let manifest = load_sync_manifest();
    let mut unsynced_added = 0u64;
    let mut unsynced_changed = 0u64;
    let mut unsynced_removed = 0u64;
    let mut total_synced_entries = 0u64;
    let mut artwork_complete = 0u64;
    let mut artwork_missing = 0u64;
    let sync_state;
    if let Some(entries) = manifest {
        let mut by_path = HashMap::new();
        for entry in entries {
            let key = entry
                .normalized_rom_path
                .clone()
                .or(entry.rom_path.clone())
                .unwrap_or_default();
            if key.is_empty() {
                continue;
            }
            if entry.last_synced_at.is_some() || entry.gamescope_entry_id.is_some() {
                total_synced_entries += 1;
            }
            match entry.artwork_status.as_deref() {
                Some("complete") => artwork_complete += 1,
                Some("missing") => artwork_missing += 1,
                _ => {}
            }
            by_path.insert(normalize_path(&key), entry);
        }
        for file in &game_files {
            match by_path.remove(&file.normalized_rom_path) {
                Some(entry) => {
                    if entry.size_bytes != 0 && entry.size_bytes != file.size_bytes {
                        unsynced_changed += 1;
                    } else if entry.mtime_ms != 0 && entry.mtime_ms != file.mtime_ms {
                        unsynced_changed += 1;
                    }
                }
                None => unsynced_added += 1,
            }
        }
        unsynced_removed = by_path.len() as u64;
        sync_state = if unsynced_added + unsynced_changed + unsynced_removed > 0 {
            "idle"
        } else {
            "idle"
        }
        .to_string();
    } else {
        total_synced_entries = gamescope_entries.min(detected_games);
        artwork_complete = storage.artwork.files.min(detected_games);
        artwork_missing = detected_games.saturating_sub(artwork_complete);
        sync_state = if detected_games == 0 {
            "idle"
        } else {
            "unknown"
        }
        .to_string();
    }
    let sync_needed =
        sync_state != "unknown" && unsynced_added + unsynced_changed + unsynced_removed > 0;
    let last_sync_state = latest_sync_summary().unwrap_or_else(|| {
        if sync_state == "unknown" {
            "unknown".to_string()
        } else if total_synced_entries > 0 {
            "success".to_string()
        } else {
            "never".to_string()
        }
    });
    let last_sync = match last_sync_state.as_str() {
        "success" => "Receipt found".to_string(),
        "error" => "Failed".to_string(),
        "running" => "Running".to_string(),
        "unknown" => "Unknown".to_string(),
        _ if total_synced_entries > 0 => "Receipt absent".to_string(),
        _ => "Never".to_string(),
    };
    let artwork_status = if artwork_complete > 0 && artwork_missing > 0 {
        format!(
            "{} complete · {} missing",
            artwork_complete, artwork_missing
        )
    } else if artwork_complete > 0 {
        format!("{} complete", artwork_complete)
    } else if sync_state == "unknown" {
        "Unknown".to_string()
    } else if detected_games > 0 {
        "0 complete".to_string()
    } else {
        "No artwork".to_string()
    };
    LibraryStatus {
        detected_games,
        detected_files: detected_games,
        gamescope_entries,
        first_sync_completed: matches!(last_sync_state.as_str(), "success" | "error") || total_synced_entries > 0,
        last_sync,
        last_sync_at: None,
        last_sync_state,
        artwork_status,
        artwork_complete,
        artwork_missing,
        sync_needed,
        sync_state,
        unsynced_added,
        unsynced_changed,
        unsynced_removed,
        total_detected_games: detected_games,
        total_synced_entries,
    }
}

fn current_game_files() -> Vec<GameFileState> {
    let mut files = Vec::new();
    for system in GAME_SYSTEMS {
        collect_game_files(
            game_system_storage_path(system).as_path(),
            system,
            &mut files,
            0,
        );
    }
    files
}

fn collect_game_files(path: &Path, _platform: &str, files: &mut Vec<GameFileState>, depth: usize) {
    if depth > 6 {
        return;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if metadata.is_dir() {
            collect_game_files(&p, _platform, files, depth + 1);
        } else if metadata.is_file() {
            let normalized_rom_path = normalize_path(&p.to_string_lossy());
            let mtime_ms = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            files.push(GameFileState {
                normalized_rom_path,
                size_bytes: metadata.len(),
                mtime_ms,
            });
        }
    }
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/").to_ascii_lowercase()
}

fn load_sync_manifest() -> Option<Vec<SyncManifestEntry>> {
    for path in SYNC_MANIFEST_PATHS {
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        if let Ok(doc) = serde_json::from_str::<SyncManifestDoc>(&text) {
            return Some(doc.entries);
        }
        if let Ok(entries) = serde_json::from_str::<Vec<SyncManifestEntry>>(&text) {
            return Some(entries);
        }
    }
    None
}

fn count_gamescope_entries() -> u64 {
    let roots = [
        "/home/owner/.local/share/applications",
        "/home/owner/.steam/steam/userdata",
    ];
    roots
        .iter()
        .map(|root| count_files_with_extension(Path::new(root), "desktop", 5))
        .sum()
}

fn count_files_with_extension(path: &Path, ext: &str, depth: usize) -> u64 {
    if depth == 0 {
        return 0;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    let mut count = 0;
    for entry in entries.flatten() {
        let p = entry.path();
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if metadata.is_dir() {
            count += count_files_with_extension(&p, ext, depth - 1);
        } else if metadata.is_file()
            && p.extension()
                .and_then(|v| v.to_str())
                .map(|v| v.eq_ignore_ascii_case(ext))
                .unwrap_or(false)
        {
            count += 1;
        }
    }
    count
}

fn latest_sync_summary() -> Option<String> {
    let paths = [
        "/var/lib/harmonia/receipts/game-sync-latest/run.json",
        "/var/lib/harmonia/receipts/homeconsole-sync-latest/run.json",
    ];
    for path in paths {
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        if text.contains("\"ok\":true") || text.contains("\"ok\": true") {
            return Some("success".to_string());
        }
        if text.contains("\"ok\":false") || text.contains("\"ok\": false") {
            return Some("error".to_string());
        }
    }
    None
}

