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
    let mut admitted_games = Vec::new();

    if let Some(entries) = manifest {
        let mut by_path = HashMap::new();
        for entry in entries.iter() {
            let key = entry
                .normalized_rom_path
                .clone()
                .or(entry.rom_path.clone())
                .unwrap_or_default();
            if key.is_empty() {
                continue;
            }
            let admitted = entry.last_synced_at.is_some() || entry.gamescope_entry_id.is_some();
            if admitted {
                total_synced_entries += 1;
                admitted_games.push(admitted_game_from_manifest(entry, &key));
            }
            match entry.artwork_status.as_deref() {
                Some("complete") => artwork_complete += 1,
                Some("missing") => artwork_missing += 1,
                _ => {}
            }
            by_path.insert(normalize_path(&key), entry.clone());
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
        sync_state = "idle".to_string();
    } else {
        total_synced_entries = gamescope_entries.min(detected_games);
        admitted_games = admitted_games_from_live_files(&game_files, &gamescope_inventory.entries);
        artwork_complete = admitted_games
            .iter()
            .filter(|game| game.artwork_paired)
            .count() as u64;
        if artwork_complete == 0 && storage.artwork.files > 0 {
            artwork_complete = storage.artwork.files.min(detected_games);
        }
        artwork_missing = detected_games.saturating_sub(artwork_complete);
        sync_state = if detected_games == 0 {
            "idle"
        } else {
            "unknown"
        }
        .to_string();
    }

    if admitted_games.len() as u64 > total_synced_entries {
        total_synced_entries = admitted_games.len() as u64;
    }
    let game_system_tally = tally_by_system(&admitted_games);
    let artwork_paired_total = admitted_games
        .iter()
        .filter(|game| game.artwork_paired)
        .count() as u64;
    if artwork_paired_total > 0 {
        artwork_complete = artwork_paired_total;
        artwork_missing = total_synced_entries.saturating_sub(artwork_complete);
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
            "{} paired · {} missing",
            artwork_complete, artwork_missing
        )
    } else if artwork_complete > 0 {
        format!("{} paired", artwork_complete)
    } else if sync_state == "unknown" {
        "Unknown".to_string()
    } else if detected_games > 0 {
        "0 paired".to_string()
    } else {
        "No artwork".to_string()
    };
    LibraryStatus {
        detected_games,
        detected_files: detected_games,
        gamescope_entries,
        gamescope_profiles: gamescope_inventory.profiles,
        gamescope_installed_games: gamescope_inventory.entries,
        game_system_tally,
        admitted_games,
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
        skipped_games: unsynced_removed,
        failed_games: if matches!(latest_sync_summary().as_deref(), Some("error")) { 1 } else { 0 },
        artwork_paired_total,
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
                system: _platform.to_string(),
                title: title_from_path(&p),
                size_bytes: metadata.len(),
                mtime_ms,
            });
        }
    }
}

fn admitted_games_from_live_files(
    files: &[GameFileState],
    gamescope_entries: &[GameScopeInstalledGame],
) -> Vec<AdmittedGameTally> {
    files
        .iter()
        .enumerate()
        .map(|(index, file)| {
            let steam_entry = gamescope_entries
                .get(index)
                .map(|entry| entry.name.clone())
                .unwrap_or_else(|| "Pending Steam entry".to_string());
            let game_id = slugify(&file.title);
            let artwork_paired = artwork_exists_for_title(&file.system, &file.title);
            AdmittedGameTally {
                title: file.title.clone(),
                system: display_system(&file.system).to_string(),
                source_file: file.normalized_rom_path.clone(),
                game_id,
                runner: runner_for_system(&file.system).to_string(),
                steam_entry,
                artwork_paired,
                artwork_source: if artwork_paired { "paired" } else { "missing" }.to_string(),
            }
        })
        .collect()
}

fn admitted_game_from_manifest(entry: &SyncManifestEntry, key: &str) -> AdmittedGameTally {
    let system = entry
        .system
        .clone()
        .or_else(|| system_from_path(key))
        .unwrap_or_else(|| "game".to_string());
    let title = entry
        .title
        .clone()
        .unwrap_or_else(|| title_from_path(Path::new(key)));
    let game_id = entry.slug.clone().unwrap_or_else(|| slugify(&title));
    let artwork_paired = matches!(entry.artwork_status.as_deref(), Some("complete"))
        || artwork_exists(&system, &game_id)
        || artwork_exists_for_title(&system, &title);
    AdmittedGameTally {
        title,
        system: display_system(&system).to_string(),
        source_file: key.to_string(),
        game_id,
        runner: entry
            .runner
            .clone()
            .unwrap_or_else(|| runner_for_system(&system).to_string()),
        steam_entry: entry
            .gamescope_entry_id
            .clone()
            .unwrap_or_else(|| "Steam entry pending".to_string()),
        artwork_paired,
        artwork_source: entry
            .artwork_source
            .clone()
            .unwrap_or_else(|| if artwork_paired { "paired" } else { "missing" }.to_string()),
    }
}

fn tally_by_system(admitted_games: &[AdmittedGameTally]) -> Vec<GameSystemTally> {
    GAME_SYSTEMS
        .iter()
        .filter_map(|system| {
            let display = display_system(system);
            let admitted = admitted_games
                .iter()
                .filter(|game| game.system == display)
                .count() as u64;
            if admitted == 0 {
                return None;
            }
            let artwork_paired = admitted_games
                .iter()
                .filter(|game| game.system == display && game.artwork_paired)
                .count() as u64;
            Some(GameSystemTally {
                system: display.to_string(),
                admitted,
                artwork_paired,
                artwork_missing: admitted.saturating_sub(artwork_paired),
            })
        })
        .collect()
}

fn title_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("Unknown game")
        .replace(['_', '-'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn system_from_path(path: &str) -> Option<String> {
    let normalized = path.replace('\\', "/").to_ascii_lowercase();
    GAME_SYSTEMS
        .iter()
        .find(|system| normalized.contains(&format!("/{system}/")))
        .map(|system| (*system).to_string())
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;
    for ch in value.chars().flat_map(|ch| ch.to_lowercase()) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_dash = false;
        } else if !last_dash && !slug.is_empty() {
            slug.push('-');
            last_dash = true;
        }
    }
    slug.trim_matches('-').to_string()
}

fn artwork_exists(system: &str, slug: &str) -> bool {
    if slug.is_empty() {
        return false;
    }
    let candidates = [
        Path::new(ARTWORK_ROOT).join(system).join(slug),
        Path::new(ARTWORK_ROOT).join(display_system(system).to_ascii_lowercase()).join(slug),
    ];
    candidates.iter().any(|path| artwork_cache_dir_has_image(path))
}

fn artwork_exists_for_title(system: &str, title: &str) -> bool {
    artwork_slug_candidates(title)
        .iter()
        .any(|slug| artwork_exists(system, slug))
}

fn artwork_cache_dir_has_image(path: &Path) -> bool {
    if !path.is_dir() {
        return false;
    }
    ["grid.png", "landscape.png", "hero.png", "logo.png", "icon.png"]
        .iter()
        .any(|name| path.join(name).is_file())
}

fn artwork_slug_candidates(title: &str) -> Vec<String> {
    let mut candidates = Vec::new();
    push_slug_candidate(&mut candidates, &slugify(title));

    let mut base = String::new();
    let mut depth = 0u32;
    for ch in title.chars() {
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            _ if depth == 0 => base.push(ch),
            _ => {}
        }
    }
    push_slug_candidate(&mut candidates, &slugify(base.trim()));

    if let Some(before_colon) = title.split(':').next() {
        push_slug_candidate(&mut candidates, &slugify(before_colon.trim()));
    }
    candidates
}

fn push_slug_candidate(candidates: &mut Vec<String>, slug: &str) {
    if !slug.is_empty() && !candidates.iter().any(|existing| existing == slug) {
        candidates.push(slug.to_string());
    }
}

fn display_system(system: &str) -> &str {
    match system {
        "gba" => "GBA",
        "genesis" => "Genesis",
        "snes" => "SNES",
        "nes" => "NES",
        "ps1" => "PS1",
        "n64" => "N64",
        "ps2" => "PS2",
        "sega-cd" => "Sega CD",
        "psp" => "PSP",
        "gamecube" => "GameCube",
        "wii" => "Wii",
        "dos" => "DOS",
        "arcade" => "Arcade",
        _ => "Game",
    }
}

fn runner_for_system(system: &str) -> &str {
    match system {
        "gba" => "RetroArch mGBA",
        "genesis" | "sega-cd" => "RetroArch Genesis Plus GX",
        "snes" => "RetroArch Snes9x",
        "nes" => "RetroArch Nestopia",
        "ps1" => "RetroArch Beetle PSX HW",
        "n64" => "RetroArch Mupen64Plus Next",
        "ps2" => "RetroArch Play!",
        "psp" => "RetroArch PPSSPP",
        "gamecube" | "wii" => "RetroArch Dolphin",
        "dos" => "DOSBox",
        "arcade" => "MAME / FinalBurn",
        _ => "Console runner",
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
        "/home/arcadia/.local/share/applications",
        "/home/arcadia/.steam/steam/userdata",
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
    let mut entries = Vec::new();
    let app_name_positions = find_steam_key_positions(bytes, b"AppName\0");
    for (entry_index, app_name_pos) in app_name_positions.iter().enumerate() {
        let name_start = app_name_pos + b"AppName\0".len();
        let name = read_null_terminated_string(bytes, name_start).unwrap_or_else(|| "Unknown".to_string());
        let segment_end = app_name_positions
            .get(entry_index + 1)
            .copied()
            .unwrap_or(bytes.len());
        let segment = &bytes[name_start..segment_end];
        let executable = read_field_after_key(segment, b"Exe\0");
        let launch_options = read_field_after_key(segment, b"LaunchOptions\0");
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
    }
    entries
}

fn find_steam_key_positions(bytes: &[u8], key: &[u8]) -> Vec<usize> {
    bytes
        .windows(key.len())
        .enumerate()
        .filter_map(|(index, window)| if window == key { Some(index) } else { None })
        .collect()
}

fn read_field_after_key(segment: &[u8], key: &[u8]) -> Option<String> {
    let key_pos = find_steam_key_positions(segment, key).into_iter().next()?;
    read_null_terminated_string(segment, key_pos + key.len())
}

fn read_null_terminated_string(bytes: &[u8], start: usize) -> Option<String> {
    if start >= bytes.len() {
        return None;
    }
    let end = bytes[start..]
        .iter()
        .position(|byte| *byte == 0)
        .map(|offset| start + offset)
        .unwrap_or(bytes.len());
    let mut value = String::from_utf8_lossy(&bytes[start..end]).to_string();
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        value = value[1..value.len() - 1].to_string();
    }
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn steam_user_from_shortcuts_path(path: &Path) -> Option<String> {
    path.parent()?.parent()?.file_name()?.to_str().map(|value| value.to_string())
}

fn owner_from_steam_root(root: &str) -> &'static str {
    if root.starts_with("/home/steam/") {
        "steam"
    } else if root.starts_with("/home/arcadia/") {
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

