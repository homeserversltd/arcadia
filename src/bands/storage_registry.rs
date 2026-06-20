fn storage_registry(network: &NetworkStatus) -> StorageRegistry {
    let volume = root_volume();
    let host = hostname();
    let netbios = "HOMECONSOLE";
    let ip = (network.ip_address != "—").then_some(network.ip_address.as_str());
    let game_roots = GAME_SYSTEMS
        .iter()
        .map(|platform| {
            let share = format!("games\\{}", platform);
            let smb_share = format!("games/{}", platform);
            let path = Path::new(GAMES_ROOT)
                .join(platform)
                .to_string_lossy()
                .to_string();
            GameRoot {
                id: (*platform).to_string(),
                platform: (*platform).to_uppercase(),
                display_name: platform_display_name(platform),
                path,
                samba_share_name: format!("games/{}", platform),
                windows_unc: Some(format!(r"\\{}\{}", netbios, share)),
                windows_unc_by_ip: ip.map(|addr| format!(r"\\{}\{}", addr, share)),
                smb_url: Some(format!("smb://{}/{}", host, smb_share)),
                smb_url_by_ip: ip.map(|addr| format!("smb://{}/{}", addr, smb_share)),
            }
        })
        .collect::<Vec<_>>();
    StorageRegistry {
        volumes: vec![volume],
        categories: StorageRegistryCategories {
            games: GameRootsRegistry {
                label: "Games".to_string(),
                roots: game_roots,
            },
            artwork: FolderRootsRegistry {
                label: "Artwork".to_string(),
                roots: vec![
                    folder_root(
                        "artwork-covers",
                        "Covers",
                        &format!("{}/covers", ARTWORK_ROOT),
                        "covers",
                    ),
                    folder_root(
                        "artwork-metadata",
                        "Metadata",
                        &format!("{}/metadata", ARTWORK_ROOT),
                        "metadata",
                    ),
                    folder_root(
                        "artwork-generated",
                        "Generated",
                        &format!("{}/generated", ARTWORK_ROOT),
                        "generated",
                    ),
                    folder_root("artwork-cache", "Scraper Cache", ARTWORK_ROOT, "cache"),
                ],
            },
            ai_models: FolderRootsRegistry {
                label: "AI Models".to_string(),
                roots: model_roots(),
            },
            updates: FolderRootsRegistry {
                label: "Updates".to_string(),
                roots: update_roots()
                    .iter()
                    .enumerate()
                    .map(|(i, p)| {
                        folder_root(&format!("updates-{}", i), "Update Cache", p, "cache")
                    })
                    .collect(),
            },
            logs: FolderRootsRegistry {
                label: "Logs".to_string(),
                roots: log_roots()
                    .iter()
                    .enumerate()
                    .map(|(i, p)| folder_root(&format!("logs-{}", i), "Logs", p, "logs"))
                    .collect(),
            },
            temporary: FolderRootsRegistry {
                label: "Temporary Files".to_string(),
                roots: TEMP_CLEAN_ROOTS
                    .iter()
                    .enumerate()
                    .map(|(i, p)| {
                        folder_root(&format!("temporary-{}", i), "Temporary", p, "temporary")
                    })
                    .collect(),
            },
            system: FolderRootsRegistry {
                label: "System".to_string(),
                roots: vec![
                    folder_root("system-root", "System", "/usr", "system"),
                    folder_root("system-var-lib", "Runtime State", "/var/lib", "system"),
                ],
            },
        },
    }
}

fn folder_root(id: &str, display_name: &str, path: &str, purpose: &str) -> FolderRoot {
    FolderRoot {
        id: id.to_string(),
        display_name: display_name.to_string(),
        path: path.to_string(),
        purpose: Some(purpose.to_string()),
    }
}
fn model_roots() -> Vec<FolderRoot> {
    let mut roots = MODEL_SCAN_ROOTS
        .iter()
        .enumerate()
        .map(|(index, path)| {
            folder_root(
                &format!("ai-installed-models-{}", index),
                "Installed Models",
                path,
                "installed-models",
            )
        })
        .collect::<Vec<_>>();
    roots.extend([
        folder_root(
            "ai-downloads",
            "Downloads",
            "/var/lib/arcadia/model-downloads",
            "downloads",
        ),
        folder_root(
            "ai-partial-downloads",
            "Partial Downloads",
            "/var/lib/arcadia/model-downloads/partial",
            "partial-downloads",
        ),
        folder_root(
            "ai-catalog-cache",
            "Catalog Cache",
            "/var/lib/arcadia/model-catalog",
            "catalog-cache",
        ),
    ]);
    roots
}

fn update_roots() -> Vec<&'static str> {
    vec![
        "/var/cache/pacman/pkg",
        "/var/lib/harmonia/cache",
        "/var/lib/harmonia/artifacts",
    ]
}
fn log_roots() -> Vec<&'static str> {
    vec![
        "/var/log/arcadia",
        "/var/log/homeconsole-sync",
        "/var/lib/harmonia/receipts",
    ]
}
fn partial_ai_download_roots() -> Vec<&'static str> {
    vec![
        "/var/lib/arcadia/model-downloads/partial",
        "/var/cache/arcadia/model-downloads",
    ]
}

fn root_volume() -> StorageVolume {
    let (fs_name, total, used, free, mount) =
        df_row("/").unwrap_or_else(|| (None, 0, 0, 0, "/".to_string()));
    let health = Some(
        match percent(used, total) {
            0..=74 => "ok",
            75..=89 => "warning",
            90..=97 => "warning",
            _ => "error",
        }
        .to_string(),
    );
    StorageVolume {
        id: "root".to_string(),
        label: "Console Storage".to_string(),
        mount_point: mount,
        filesystem: fs_name,
        total_bytes: total,
        used_bytes: used,
        free_bytes: free,
        health,
    }
}

fn df_row(path: &str) -> Option<(Option<String>, u64, u64, u64, String)> {
    let output = Command::new("df").args(["-B1", "-T", path]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.lines().nth(1)?;
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 7 {
        return None;
    }
    Some((
        Some(parts[1].to_string()),
        parts[2].parse().ok()?,
        parts[3].parse().ok()?,
        parts[4].parse().ok()?,
        parts[6].to_string(),
    ))
}

fn game_folder_storage(root: &GameRoot) -> GameFolderStorage {
    let usage = path_usage(Path::new(&root.path));
    let largest_files = largest_files(Path::new(&root.path), 3);
    let synced = load_sync_manifest().map(|entries| {
        entries
            .iter()
            .filter(|e| {
                e.normalized_rom_path
                    .as_ref()
                    .or(e.rom_path.as_ref())
                    .map(|p| p.to_ascii_lowercase().contains(&format!("/{}/", root.id)))
                    .unwrap_or(false)
            })
            .count() as u64
    });
    let unsynced = synced.map(|s| usage.files.saturating_sub(s));
    GameFolderStorage {
        platform: root.platform.clone(),
        display_name: root.display_name.clone(),
        path: root.path.clone(),
        samba_share_name: root.samba_share_name.clone(),
        bytes: usage.bytes,
        size: human_size(usage.bytes),
        file_count: usage.files,
        synced_entries: synced,
        unsynced_files: unsynced,
        largest_files,
        windows_unc: root.windows_unc.clone(),
        windows_unc_by_ip: root.windows_unc_by_ip.clone(),
        smb_url: root.smb_url.clone(),
        smb_url_by_ip: root.smb_url_by_ip.clone(),
    }
}

fn folder_storage(id: &str, display_name: &str, path: &str) -> FolderStorage {
    let usage = path_usage(Path::new(path));
    let state = if !Path::new(path).exists() {
        "unknown"
    } else {
        "ok"
    };
    FolderStorage {
        id: id.to_string(),
        display_name: display_name.to_string(),
        path: path.to_string(),
        bytes: usage.bytes,
        size: human_size(usage.bytes),
        file_count: usage.files,
        last_modified_at: fs::metadata(path)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(system_time_string),
        state: state.to_string(),
    }
}

