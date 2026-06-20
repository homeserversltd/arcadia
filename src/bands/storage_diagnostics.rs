fn ai_model_files() -> Vec<AIModelStorage> {
    let local_ai = local_ai_status();
    let loaded_name = local_ai.loaded_model.clone();
    let selected_id = local_ai.selected_model_id.clone();
    let mut out = Vec::new();
    for root in model_roots()
        .into_iter()
        .filter(|r| r.purpose.as_deref() == Some("installed-models"))
    {
        let mut found = Vec::new();
        collect_ai_models(Path::new(&root.path), &mut found, 0);
        for (bytes, filename, path) in found {
            let id = model_id(&filename);
            let loaded = loaded_name
                .as_ref()
                .map(|n| n == &filename)
                .unwrap_or(false);
            out.push(AIModelStorage {
                id: id.clone(),
                name: friendly_model_name(&filename),
                filename: filename.clone(),
                path: path.to_string_lossy().to_string(),
                bytes,
                size: human_size(bytes),
                source: "unknown".to_string(),
                loaded,
                selected: selected_id.as_ref().map(|s| s == &id).unwrap_or(false),
                removable: !loaded,
            });
        }
    }
    if let Some(loaded) = loaded_name.as_ref() {
        if !out.iter().any(|model| model.filename == *loaded) {
            let id = model_id(loaded);
            out.push(AIModelStorage {
                id: id.clone(),
                name: friendly_model_name(loaded),
                filename: loaded.clone(),
                path: "Unknown".to_string(),
                bytes: 0,
                size: "Unknown".to_string(),
                source: "unknown".to_string(),
                loaded: true,
                selected: selected_id.as_ref().map(|s| s == &id).unwrap_or(false),
                removable: false,
            });
        }
    }
    out.sort_by(|a, b| b.bytes.cmp(&a.bytes));
    out
}

fn category_status(
    bytes: u64,
    files: u64,
    root_count: usize,
    used: u64,
    total: u64,
    suffix: &str,
    state: &str,
    detail: &str,
) -> StorageCategoryStatus {
    StorageCategoryStatus {
        bytes,
        size: human_size(bytes),
        files,
        file_count: Some(files),
        root_count,
        state: state.to_string(),
        meta: format!("{} {} · {} roots", files, suffix, root_count),
        detail: detail.to_string(),
        percent_of_used: percent(bytes, used),
        percent_of_total: percent(bytes, total),
    }
}
fn state_for_roots(roots: &[FolderStorage]) -> &str {
    if roots.iter().any(|r| r.state == "unknown") {
        "unknown"
    } else {
        "ok"
    }
}

fn largest_game_files(path: &Path, platform: &str, limit: usize) -> Vec<LargestFile> {
    let mut files = Vec::new();
    collect_largest_game_files(path, platform, &mut files, 0);
    files.sort_by(|a, b| b.0.cmp(&a.0));
    files
        .into_iter()
        .take(limit)
        .map(|(bytes, path)| LargestFile {
            name: path
                .file_name()
                .and_then(|v| v.to_str())
                .unwrap_or("file")
                .to_string(),
            path: path.to_string_lossy().to_string(),
            bytes,
            size: human_size(bytes),
        })
        .collect()
}
fn collect_largest_game_files(
    path: &Path,
    platform: &str,
    out: &mut Vec<(u64, PathBuf)>,
    depth: usize,
) {
    if depth > 8 {
        return;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        let Ok(m) = e.metadata() else {
            continue;
        };
        if m.is_dir() {
            collect_largest_game_files(&p, platform, out, depth + 1);
        } else if m.is_file() && is_playable_game_file(&p, platform) {
            out.push((m.len(), p));
        }
    }
}

fn game_path_usage(path: &Path, platform: &str, depth: usize) -> Usage {
    if depth > 8 {
        return Usage::default();
    }
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return Usage::default();
    };
    if metadata.file_type().is_symlink() {
        return Usage::default();
    }
    if metadata.is_file() {
        return if is_playable_game_file(path, platform) {
            Usage {
                bytes: metadata.len(),
                files: 1,
            }
        } else {
            Usage::default()
        };
    }
    if !metadata.is_dir() {
        return Usage::default();
    }
    let mut usage = Usage::default();
    let Ok(entries) = fs::read_dir(path) else {
        return usage;
    };
    for entry in entries.flatten() {
        let child = game_path_usage(&entry.path(), platform, depth + 1);
        usage.bytes = usage.bytes.saturating_add(child.bytes);
        usage.files = usage.files.saturating_add(child.files);
    }
    usage
}

fn is_playable_game_file(path: &Path, platform: &str) -> bool {
    let ext = path
        .extension()
        .and_then(|v| v.to_str())
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_default();
    let allowed: &[&str] = match platform {
        "gba" => &["gba"],
        "genesis" => &["md", "gen", "smd", "bin"],
        "snes" => &["sfc", "smc"],
        "nes" => &["nes"],
        "ps1" => &["cue", "chd", "iso", "pbp"],
        "n64" => &["z64", "n64", "v64"],
        "ps2" => &["iso", "chd", "cso", "bin"],
        "sega-cd" => &["cue", "chd", "iso"],
        "psp" => &["iso", "cso", "pbp"],
        "gamecube" => &["iso", "gcm", "rvz", "ciso"],
        "wii" => &["iso", "wbfs", "rvz"],
        "dos" => &["conf", "bat", "exe"],
        _ => &[],
    };
    allowed.iter().any(|candidate| *candidate == ext)
}

fn storage_diagnostics(
    registry: &StorageRegistry,
    game_folders: &[GameFolderStorage],
    games_bytes: u64,
    duration: u64,
    volume: &StorageVolume,
) -> StorageDiagnostics {
    let managed = managed_storage_paths_from_registry(registry);
    let missing_dirs = managed
        .iter()
        .filter(|p| !Path::new(p.as_str()).exists())
        .cloned()
        .collect::<Vec<_>>();
    let permission_errors = managed
        .iter()
        .filter(|p| Path::new(p.as_str()).exists() && fs::read_dir(p).is_err())
        .cloned()
        .collect::<Vec<_>>();
    let mut warnings = Vec::new();
    let manifest_entries = load_sync_manifest()
        .map(|entries| entries.len())
        .unwrap_or(0);
    if manifest_entries > 0 && games_bytes == 0 {
        warnings.push(
            "Game library has entries, but managed game folders scanned as empty.".to_string(),
        );
    }
    let samba_roots = registry.categories.games.roots.len();
    if samba_roots != game_folders.len() {
        warnings.push("Samba game folders and Storage game folders do not match.".to_string());
    }
    if !permission_errors.is_empty() && games_bytes == 0 {
        warnings.push("Game folders scanned as empty while read errors exist.".to_string());
    }
    let overlap_warnings = storage_overlap_warnings(&managed);
    let category_scan_errors = permission_errors
        .iter()
        .map(|p| format!("Could not scan this folder. Permission denied: {}", p))
        .collect::<Vec<_>>();
    StorageDiagnostics {
        mount_point: volume.mount_point.clone(),
        filesystem: volume.filesystem.clone(),
        scan_duration_ms: duration,
        scanner_version: "arcadia.storage.scan.v2".to_string(),
        missing_dirs,
        permission_errors,
        warnings,
        overlap_warnings,
        category_scan_errors,
        last_scan_timestamp: now_rfc3339_like(),
    }
}

fn storage_overlap_warnings(paths: &[String]) -> Vec<String> {
    let mut warnings = Vec::new();
    for (i, a) in paths.iter().enumerate() {
        for b in paths.iter().skip(i + 1) {
            let a = a.trim_end_matches('/');
            let b = b.trim_end_matches('/');
            if a != b && (b.starts_with(&format!("{}/", a)) || a.starts_with(&format!("{}/", b))) {
                warnings.push(format!("Managed storage roots overlap: {} and {}", a, b));
            }
        }
    }
    warnings
}
fn managed_storage_paths_from_registry(registry: &StorageRegistry) -> Vec<String> {
    let mut paths = Vec::new();
    paths.extend(
        registry
            .categories
            .games
            .roots
            .iter()
            .map(|r| r.path.clone()),
    );
    for cat in [
        &registry.categories.artwork,
        &registry.categories.ai_models,
        &registry.categories.updates,
        &registry.categories.logs,
        &registry.categories.temporary,
        &registry.categories.system,
    ] {
        paths.extend(cat.roots.iter().map(|r| r.path.clone()));
    }
    paths
}
fn is_managed_storage_path(path: &str) -> bool {
    managed_storage_paths_from_registry(&storage_registry(&network_status()))
        .iter()
        .any(|p| p == path)
}

fn cleanup_known_roots(
    action: &'static str,
    roots: Vec<&'static str>,
    success: &str,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    let mut removed = 0u64;
    let mut errors = Vec::new();
    for root in roots {
        match remove_children(Path::new(root)) {
            Ok(count) => removed += count,
            Err(err) => errors.push(format!("{}: {}", root, err)),
        }
    }
    if errors.is_empty() {
        (
            StatusCode::OK,
            Json(ConsoleActionResponse {
                ok: true,
                action,
                command: "arcadia-storage",
                exit_code: Some(0),
                message: format!("{} Removed {} entries.", success, removed),
                stdout: String::new(),
                stderr: String::new(),
            }),
        )
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ConsoleActionResponse {
                ok: false,
                action,
                command: "arcadia-storage",
                exit_code: Some(1),
                message: "Cleanup partially failed.".to_string(),
                stdout: String::new(),
                stderr: errors.join("\n"),
            }),
        )
    }
}

fn platform_display_name(platform: &str) -> String {
    match platform {
        "gba" => "GBA",
        "snes" => "SNES",
        "nes" => "NES",
        "n64" => "N64",
        "ps1" => "PS1",
        "ps2" => "PS2",
        "psp" => "PSP",
        "wii" => "Wii",
        "sega-cd" => "Sega CD",
        "gamecube" => "GameCube",
        "genesis" => "Genesis",
        "dos" => "DOS",
        other => other,
    }
    .to_string()
}
