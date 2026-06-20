fn library_status(storage: &StorageStatus) -> LibraryStatus {
    let game_files = current_game_files();
    let detected_games = game_files.len() as u64;
    let gamescope_inventory = gamescope_inventory();
    let gamescope_entries = gamescope_inventory.entries.len() as u64;
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
        gamescope_profiles: gamescope_inventory.profiles,
        gamescope_installed_games: gamescope_inventory.entries,
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
        } else if metadata.is_file() && is_playable_game_file(&p, _platform) {
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

fn gamescope_inventory() -> GameScopeInventory {
    let desktop_entries = desktop_gamescope_entries();
    let mut profiles = Vec::new();
    let mut entries = desktop_entries;
    for root in STEAM_USERDATA_ROOTS {
        collect_steam_shortcuts_profiles(Path::new(root), root, &mut profiles, &mut entries, 6);
    }
    GameScopeInventory { profiles, entries }
}

fn desktop_gamescope_entries() -> Vec<GameScopeInstalledGame> {
    let mut entries = Vec::new();
    for root in [
        "/home/owner/.local/share/applications",
        "/home/owner/.steam/steam/userdata",
    ] {
        collect_desktop_gamescope_entries(Path::new(root), root, &mut entries, 5);
    }
    entries
}

fn collect_desktop_gamescope_entries(
    path: &Path,
    root: &str,
    out: &mut Vec<GameScopeInstalledGame>,
    depth: usize,
) {
    if depth == 0 {
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
            collect_desktop_gamescope_entries(&p, root, out, depth - 1);
        } else if metadata.is_file()
            && p.extension()
                .and_then(|v| v.to_str())
                .map(|v| v.eq_ignore_ascii_case("desktop"))
                .unwrap_or(false)
        {
            let name = p
                .file_stem()
                .and_then(|v| v.to_str())
                .unwrap_or("desktop-entry")
                .to_string();
            out.push(GameScopeInstalledGame {
                name,
                steam_user: "desktop".to_string(),
                owner_user: owner_from_steam_root(root).to_string(),
                source_root: root.to_string(),
                shortcuts_vdf: p.to_string_lossy().to_string(),
                entry_index: out.len() as u64,
                executable: None,
                launch_options: None,
            });
        }
    }
}

fn collect_steam_shortcuts_profiles(
    path: &Path,
    root: &str,
    profiles: &mut Vec<GameScopeProfileInventory>,
    entries: &mut Vec<GameScopeInstalledGame>,
    depth: usize,
) {
    if depth == 0 {
        return;
    }
    let Ok(children) = fs::read_dir(path) else {
        return;
    };
    for child in children.flatten() {
        let p = child.path();
        let Ok(metadata) = child.metadata() else {
            continue;
        };
        if metadata.is_dir() {
            collect_steam_shortcuts_profiles(&p, root, profiles, entries, depth - 1);
        } else if metadata.is_file()
            && p.file_name()
                .and_then(|v| v.to_str())
                .map(|v| v.eq_ignore_ascii_case("shortcuts.vdf"))
                .unwrap_or(false)
        {
            let steam_user = steam_user_from_shortcuts_path(&p).unwrap_or_else(|| "unknown".to_string());
            let owner_user = owner_from_steam_root(root).to_string();
            let parsed = read_steam_shortcuts_file(&p, root, &owner_user, &steam_user);
            profiles.push(GameScopeProfileInventory {
                steam_user,
                owner_user,
                source_root: root.to_string(),
                shortcuts_vdf: p.to_string_lossy().to_string(),
                installed_count: parsed.len() as u64,
            });
            entries.extend(parsed);
        }
    }
}

fn read_steam_shortcuts_file(
    path: &Path,
    root: &str,
    owner_user: &str,
    steam_user: &str,
) -> Vec<GameScopeInstalledGame> {
    let Ok(bytes) = fs::read(path) else {
        return Vec::new();
    };
    parse_steam_shortcuts_bytes(
        &bytes,
        root,
        owner_user,
        steam_user,
        &path.to_string_lossy(),
    )
}

fn parse_steam_shortcuts_bytes(
    bytes: &[u8],
    root: &str,
    owner_user: &str,
    steam_user: &str,
    shortcuts_vdf: &str,
) -> Vec<GameScopeInstalledGame> {
    let tokens = bytes
        .split(|byte| *byte == 0)
        .filter_map(|token| std::str::from_utf8(token).ok())
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    let mut entries = Vec::new();
    let mut i = 0usize;
    while i < tokens.len() {
        if tokens[i] == "AppName" {
            let name = tokens.get(i + 1).unwrap_or(&"Unknown").to_string();
            let mut executable = None;
            let mut launch_options = None;
            let mut j = i + 2;
            while j < tokens.len() && tokens[j] != "AppName" {
                match tokens[j] {
                    "Exe" => executable = tokens.get(j + 1).map(|value| value.to_string()),
                    "LaunchOptions" => {
                        launch_options = tokens.get(j + 1).map(|value| value.to_string())
                    }
                    _ => {}
                }
                j += 1;
            }
            entries.push(GameScopeInstalledGame {
                name,
                steam_user: steam_user.to_string(),
                owner_user: owner_user.to_string(),
                source_root: root.to_string(),
                shortcuts_vdf: shortcuts_vdf.to_string(),
                entry_index: entries.len() as u64,
                executable,
                launch_options,
            });
            i = j;
        } else {
            i += 1;
        }
    }
    entries
}

fn steam_user_from_shortcuts_path(path: &Path) -> Option<String> {
    path.parent()?.parent()?.file_name()?.to_str().map(|value| value.to_string())
}

fn owner_from_steam_root(root: &str) -> &'static str {
    if root.starts_with("/home/steam/") {
        "steam"
    } else if root.starts_with("/home/owner/") {
        "owner"
    } else {
        "unknown"
    }
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

