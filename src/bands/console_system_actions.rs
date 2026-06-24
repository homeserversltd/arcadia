async fn system_status_route(State(state): State<Arc<AppState>>) -> Json<SystemAdminStatus> {
    let status = console_status(&state);
    Json(status.system)
}

async fn action_restart_arcadia(
    Json(body): Json<ConsoleActionRequest>,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    if body.confirm.as_deref() != Some("RESTART_ARCADIA") {
        return console_action_error(
            StatusCode::BAD_REQUEST,
            "restart-arcadia",
            SYSTEMCTL_BIN,
            "Confirm Arcadia restart before restarting the web GUI.",
        );
    }
    run_console_command(
        "restart-arcadia",
        SYSTEMCTL_BIN,
        &["restart", "arcadia.service"],
        "Arcadia restart requested.",
        "Arcadia restart request failed.",
    )
}

async fn action_ssh_service(
    Json(body): Json<TrustModeRequest>,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    match body.mode.as_str() {
        "enable" => run_console_command(
            "enable-ssh",
            SYSTEMCTL_BIN,
            &["enable", "--now", "sshd.service"],
            "SSH service enabled.",
            "SSH service could not be enabled.",
        ),
        "disable" => {
            if body.confirm.as_deref() != Some("DISABLE_SSH") {
                return console_action_error(
                    StatusCode::BAD_REQUEST,
                    "disable-ssh",
                    SYSTEMCTL_BIN,
                    "Confirm before disabling SSH service.",
                );
            }
            run_console_command(
                "disable-ssh",
                SYSTEMCTL_BIN,
                &["disable", "--now", "sshd.service"],
                "SSH service disabled.",
                "SSH service could not be disabled.",
            )
        }
        _ => console_action_error(
            StatusCode::BAD_REQUEST,
            "ssh-service",
            SYSTEMCTL_BIN,
            "SSH service mode must be enable or disable.",
        ),
    }
}

async fn action_ssh_password_auth(
    Json(body): Json<TrustModeRequest>,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    match body.mode.as_str() {
        "enable" => {
            if body.confirm.as_deref() != Some("ENABLE_SSH_PASSWORD") {
                return console_action_error(
                    StatusCode::BAD_REQUEST,
                    "enable-ssh-password",
                    SSHD_CONFIG_MANAGED_PATH,
                    "Confirm before enabling SSH password login.",
                );
            }
            write_ssh_password_auth(true)
        }
        "disable" => {
            if body.confirm.as_deref() != Some("DISABLE_SSH_PASSWORD") {
                return console_action_error(
                    StatusCode::BAD_REQUEST,
                    "disable-ssh-password",
                    SSHD_CONFIG_MANAGED_PATH,
                    "Confirm before disabling SSH password login.",
                );
            }
            write_ssh_password_auth(false)
        }
        _ => console_action_error(
            StatusCode::BAD_REQUEST,
            "ssh-password-auth",
            SSHD_CONFIG_MANAGED_PATH,
            "SSH password mode must be enable or disable.",
        ),
    }
}

async fn action_install_authorized_key(
    Json(body): Json<SshKeyInstallRequest>,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    install_authorized_key(&body.public_key)
}

async fn action_install_root_ca(
    Json(body): Json<RootCaInstallRequest>,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    if body.confirm.as_deref() != Some("INSTALL_ROOT_CA") {
        return console_action_error(
            StatusCode::BAD_REQUEST,
            "install-root-ca",
            HOMECONSOLE_CA_ANCHOR_PATH,
            "Confirm before installing the Home Root CA.",
        );
    }
    install_root_ca(&body.ca_bundle)
}

async fn action_set_trust_mode(
    Json(body): Json<TrustModeRequest>,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    set_trust_mode(&body.mode, body.confirm.as_deref())
}

async fn action_update_gui() -> (StatusCode, Json<ConsoleActionResponse>) {
    run_console_command(
        "update-gui",
        SYSTEMD_RUN_BIN,
        &[
            "--unit=arcadia-gui-update",
            "--collect",
            HARMONIA_BIN,
            "homeconsole-arcadia-gui-update",
            HOMECONSOLE_PROFILE,
            "--repo",
            "git@git.home.arpa:HOMESERVERSLTD/arcadia.git",
            "--branch",
            "main",
            "--source-dir",
            "/opt/arcadia/source",
            "--apply",
            "--install-bin",
            "/usr/local/bin/arcadia",
            "--service",
            "arcadia.service",
            "--receipt-dir",
            "/var/lib/harmonia/receipts/arcadia-gui-latest",
        ],
        "Update GUI started. Read /var/lib/harmonia/receipts/arcadia-gui-latest after Arcadia restarts.",
        "Update GUI could not start.",
    )
}


async fn action_check_updates() -> (StatusCode, Json<ConsoleActionResponse>) {
    run_caduceus_http_mutation(
        "check-updates",
        "/api/v1/update/check",
        "Harmonia check complete.",
        "Harmonia check failed. Read /var/lib/harmonia/receipts/homeconsole-check-latest.",
    )
}

async fn action_harmonia_module_toggle(
    Json(body): Json<HarmoniaModuleToggleRequest>,
) -> (StatusCode, Json<HarmoniaModuleToggleResponse>) {
    let module_id = body.module_id.trim();
    if !valid_harmonia_module_id(module_id) {
        return harmonia_module_toggle_response(
            StatusCode::BAD_REQUEST,
            false,
            module_id.to_string(),
            body.enabled,
            "Module id must be lowercase letters, numbers, and hyphens only.".to_string(),
        );
    }
    let profile_path = Path::new(HOMECONSOLE_PROFILE);
    let Ok(text) = fs::read_to_string(profile_path) else {
        return harmonia_module_toggle_response(
            StatusCode::SERVICE_UNAVAILABLE,
            false,
            module_id.to_string(),
            body.enabled,
            "Harmonia profile is not readable on this console.".to_string(),
        );
    };
    let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&text) else {
        return harmonia_module_toggle_response(
            StatusCode::CONFLICT,
            false,
            module_id.to_string(),
            body.enabled,
            "Harmonia profile JSON is invalid; module state was not changed.".to_string(),
        );
    };
    let modules_value = json
        .get_mut("modules")
        .and_then(|modules| modules.as_array_mut());
    let Some(modules) = modules_value else {
        return harmonia_module_toggle_response(
            StatusCode::CONFLICT,
            false,
            module_id.to_string(),
            body.enabled,
            "Harmonia profile has no modules array; module state was not changed.".to_string(),
        );
    };

    let had_module = modules.iter().any(|module| module.as_str() == Some(module_id));
    if body.enabled && !had_module {
        modules.push(serde_json::Value::String(module_id.to_string()));
    } else if !body.enabled {
        modules.retain(|module| module.as_str() != Some(module_id));
    }

    let rendered = match serde_json::to_string_pretty(&json) {
        Ok(rendered) => rendered + "\n",
        Err(_) => {
            return harmonia_module_toggle_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                false,
                module_id.to_string(),
                body.enabled,
                "Harmonia profile could not be rendered.".to_string(),
            );
        }
    };
    let backup_path = profile_path.with_extension("index.json.arcadia-bak");
    let _ = fs::write(&backup_path, text.as_bytes());
    if let Err(err) = fs::write(profile_path, rendered.as_bytes()) {
        return harmonia_module_toggle_response(
            StatusCode::SERVICE_UNAVAILABLE,
            false,
            module_id.to_string(),
            body.enabled,
            format!("Harmonia profile could not be saved: {err}"),
        );
    }
    harmonia_module_toggle_response(
        StatusCode::OK,
        true,
        module_id.to_string(),
        body.enabled,
        if body.enabled {
            format!("{} enabled for the next Harmonia run.", module_id)
        } else {
            format!("{} disabled for the next Harmonia run.", module_id)
        },
    )
}

fn valid_harmonia_module_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 96
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn harmonia_module_toggle_response(
    status: StatusCode,
    ok: bool,
    module_id: String,
    enabled: bool,
    message: String,
) -> (StatusCode, Json<HarmoniaModuleToggleResponse>) {
    (
        status,
        Json(HarmoniaModuleToggleResponse {
            ok,
            action: "harmonia-module-toggle",
            module_id,
            enabled,
            profile_path: HOMECONSOLE_PROFILE,
            message,
        }),
    )
}


async fn harmonia_ledger_route(
    Query(query): Query<HarmoniaLedgerQuery>,
) -> (StatusCode, Json<HarmoniaLedgerResponse>) {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(10).clamp(1, 25);
    let path = format!("/api/v1/receipts/ledger?page={page}&per_page={per_page}");
    let Ok(value) = caduceus_fetch_json(&path) else {
        return (
            StatusCode::BAD_GATEWAY,
            Json(HarmoniaLedgerResponse {
                ok: false,
                action: "harmonia-ledger-page",
                profile_id: "homeconsole",
                ledger_path: HARMONIA_HOMECONSOLE_LEDGER,
                page,
                per_page,
                total_entries: 0,
                total_pages: 1,
                entries: Vec::new(),
                message: "Caduceus ledger route is unreachable.".to_string(),
            }),
        );
    };
    let _ledger_path = value
        .get("ledgerPath")
        .and_then(|v| v.as_str())
        .unwrap_or(HARMONIA_HOMECONSOLE_LEDGER);
    let total_entries = value
        .get("totalEntries")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let total_pages = value
        .get("totalPages")
        .and_then(|v| v.as_u64())
        .unwrap_or(1) as usize;
    let bounded_page = value
        .get("page")
        .and_then(|v| v.as_u64())
        .unwrap_or(page as u64) as usize;
    let entries = value
        .get("entries")
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let ordinal = item.get("ordinal").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                    let mut entry = item.get("entry").cloned().unwrap_or(serde_json::Value::Null);
                    redact_json_value(&mut entry);
                    Some(harmonia_ledger_entry(ordinal, entry))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let message = if total_entries == 0 {
        "No Harmonia ledger entries found.".to_string()
    } else {
        format!("Showing ledger page {} of {}.", bounded_page, total_pages)
    };
    (
        StatusCode::OK,
        Json(HarmoniaLedgerResponse {
            ok: true,
            action: "harmonia-ledger-page",
            profile_id: "homeconsole",
            ledger_path: HARMONIA_HOMECONSOLE_LEDGER,
            page: bounded_page,
            per_page,
            total_entries,
            total_pages,
            entries,
            message,
        }),
    )
}

fn harmonia_ledger_entry(ordinal: usize, entry: serde_json::Value) -> HarmoniaLedgerEntry {
    HarmoniaLedgerEntry {
        ordinal,
        stamp: json_string_any(&entry, &["stamp", "timestamp", "completed_at", "started_at", "created_at"])
            .unwrap_or_else(|| format!("entry-{ordinal}")),
        schema: json_string_any(&entry, &["schema"]).unwrap_or_else(|| "harmonia.ledger.entry".to_string()),
        profile_id: json_string_any(&entry, &["profile_id", "profileId", "profile"]).unwrap_or_else(|| "homeconsole".to_string()),
        module_id: json_string_any(&entry, &["module_id", "moduleId", "module"]).unwrap_or_else(|| "suite".to_string()),
        ok: entry.get("ok").and_then(|v| v.as_bool()),
        changed: entry.get("changed").and_then(|v| v.as_bool()),
        first_missing_signal: json_string_any(&entry, &["first_missing_signal", "firstMissingSignal"])
            .unwrap_or_else(|| "none".to_string()),
        receipt_dir: json_string_any(&entry, &["receipt_dir", "receiptDir"]).unwrap_or_default(),
        entry,
    }
}

fn json_string_any(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(|v| v.as_str()).map(str::to_string))
}

fn redact_json_value(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, value) in map.iter_mut() {
                let key_lc = key.to_ascii_lowercase();
                if key_lc.contains("secret")
                    || key_lc.contains("token")
                    || key_lc.contains("password")
                    || key_lc.contains("private")
                    || key_lc.ends_with("key")
                {
                    *value = serde_json::Value::String("redacted".to_string());
                } else {
                    redact_json_value(value);
                }
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                redact_json_value(item);
            }
        }
        _ => {}
    }
}

async fn action_sync_games() -> (StatusCode, Json<ConsoleActionResponse>) {
    run_caduceus_http_mutation(
        "sync-games",
        "/api/v1/sync/now",
        "Games synced. Receipt ready.",
        "Sync failed. Open the ledger for the reason and fix action.",
    )
}

const MAX_SYNC_UPLOAD_FILES: u64 = 32;
const MAX_SYNC_UPLOAD_BYTES: usize = 256 * 1024 * 1024;
const MAX_SYNC_UPLOAD_TOTAL_BYTES: usize = 512 * 1024 * 1024;

async fn action_add_games_upload(mut multipart: Multipart) -> (StatusCode, Json<ConsoleActionResponse>) {
    let mut accepted = 0u64;
    let mut rejected = 0u64;
    let mut selected_system: Option<String> = None;
    let mut seen = Vec::new();
    let mut total_bytes = 0usize;
    let mut staged = Vec::new();
    let mut complaints = Vec::new();

    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => break,
            Err(_) => {
                rejected += 1;
                complaints.push("The upload could not be read.".to_string());
                break;
            }
        };
        let field_name = field.name().unwrap_or("").to_string();
        if field_name == "system" {
            match field.text().await {
                Ok(value) if valid_game_system(value.trim()) => {
                    selected_system = Some(value.trim().to_string());
                }
                _ => {
                    rejected += 1;
                    complaints.push("Select a supported game kind before adding files.".to_string());
                }
            }
            continue;
        }
        if field_name != "games" {
            continue;
        }
        let Some(system) = selected_system.as_deref() else {
            rejected += 1;
            complaints.push("Select a game kind before adding files.".to_string());
            continue;
        };
        if accepted + rejected >= MAX_SYNC_UPLOAD_FILES {
            rejected += 1;
            complaints.push("Too many files were added at once.".to_string());
            continue;
        }
        let Some(file_name) = field.file_name().map(|name| name.to_string()) else {
            rejected += 1;
            complaints.push("A file was rejected because it had no name.".to_string());
            continue;
        };
        let safe_name = safe_upload_name(&file_name);
        if seen.iter().any(|existing| existing == &safe_name) {
            rejected += 1;
            complaints.push(format!("{} was rejected: duplicate name.", public_file_name(&safe_name)));
            continue;
        }
        seen.push(safe_name.clone());
        if !supported_game_file(&safe_name) {
            rejected += 1;
            complaints.push(format!("{} was rejected: unsupported game file.", public_file_name(&safe_name)));
            continue;
        }
        let Ok(bytes) = field.bytes().await else {
            rejected += 1;
            complaints.push(format!("{} was rejected: the machine could not read it.", public_file_name(&safe_name)));
            continue;
        };
        if bytes.is_empty() {
            rejected += 1;
            complaints.push(format!("{} was rejected: empty file.", public_file_name(&safe_name)));
            continue;
        }
        if bytes.len() > MAX_SYNC_UPLOAD_BYTES {
            rejected += 1;
            complaints.push(format!("{} was rejected: file is too large for this intake lane.", public_file_name(&safe_name)));
            continue;
        }
        total_bytes = total_bytes.saturating_add(bytes.len());
        if total_bytes > MAX_SYNC_UPLOAD_TOTAL_BYTES {
            rejected += 1;
            complaints.push(format!("{} was rejected: this upload batch is too large.", public_file_name(&safe_name)));
            continue;
        }
        let target_dir = match sync_upload_target_dir(system) {
            Ok(path) => path,
            Err(err) => {
                rejected += 1;
                complaints.push(format!("{} was rejected: {err}.", public_file_name(&safe_name)));
                continue;
            }
        };
        if let Err(err) = fs::create_dir_all(&target_dir) {
            rejected += 1;
            complaints.push(format!("{} was rejected: storage is not ready ({err}).", public_file_name(&safe_name)));
            continue;
        }
        let target = target_dir.join(&safe_name);
        match OpenOptions::new().write(true).create_new(true).open(&target) {
            Ok(mut file) => match file.write_all(&bytes) {
                Ok(()) => {
                    accepted += 1;
                    staged.push(serde_json::json!({
                        "display_name": public_file_name(&safe_name),
                        "system": platform_display_name(system),
                        "state": "placed"
                    }));
                }
                Err(err) => {
                    let _ = fs::remove_file(&target);
                    rejected += 1;
                    complaints.push(format!("{} was rejected: storage would not accept it ({err}).", public_file_name(&safe_name)));
                }
            },
            Err(err) => {
                rejected += 1;
                complaints.push(format!("{} was rejected: a game with that name already exists or storage refused it ({err}).", public_file_name(&safe_name)));
            }
        }
    }

    if selected_system.is_none() {
        rejected = rejected.saturating_add(1);
        complaints.push("Select a game kind before adding files.".to_string());
    }
    if accepted == 0 && rejected == 0 {
        rejected = 1;
        complaints.push("No game files were selected.".to_string());
    }

    let ok = accepted > 0 && rejected == 0;
    let partial = accepted > 0 && rejected > 0;
    let receipt = serde_json::json!({
        "family": "arcadia.sync.upload.v1",
        "ok": ok || partial,
        "selected_system": selected_system.as_deref().map(platform_display_name),
        "accepted": accepted,
        "rejected": rejected,
        "staged": staged,
        "complaints": complaints,
    });
    let _ = fs::create_dir_all("/var/lib/arcadia/sync-upload-latest");
    let _ = fs::write(
        "/var/lib/arcadia/sync-upload-latest/run.json",
        serde_json::to_string_pretty(&receipt).unwrap_or_else(|_| "{}".to_string()),
    );

    let status = if accepted > 0 { StatusCode::OK } else { StatusCode::BAD_REQUEST };
    let message = if partial {
        format!("{} accepted · {} rejected. Open the ledger for the reason and fix action.", accepted, rejected)
    } else if accepted > 0 {
        format!("{} game{} staged for {}. Press Sync games.", accepted, if accepted == 1 { "" } else { "s" }, selected_system.as_deref().map(platform_display_name).unwrap_or_else(|| "the selected kind".to_string()))
    } else {
        "Those files were rejected. Open the ledger for the reason and fix action.".to_string()
    };

    (
        status,
        Json(ConsoleActionResponse {
            ok: accepted > 0,
            action: "add-games",
            command: "arcadia-sync-intake",
            exit_code: Some(if accepted > 0 { 0 } else { 1 }),
            message,
            stdout: format!("accepted={accepted} rejected={rejected}"),
            stderr: complaints.join("\n"),
        }),
    )
}

fn sync_upload_target_dir(system: &str) -> Result<PathBuf, String> {
    if !valid_game_system(system) {
        return Err("unsupported game kind".to_string());
    }
    let target_dir = game_system_storage_path(system);
    fs::create_dir_all(&target_dir).map_err(|_| "storage is not ready".to_string())?;
    let games_root = Path::new(GAMES_ROOT)
        .canonicalize()
        .map_err(|_| "game storage is not ready".to_string())?;
    let canonical_target = target_dir
        .canonicalize()
        .map_err(|_| "game storage is not ready".to_string())?;
    if !canonical_target.starts_with(&games_root) {
        return Err("storage is not safe for intake".to_string());
    }
    Ok(canonical_target)
}

fn valid_game_system(system: &str) -> bool {
    GAME_SYSTEMS.contains(&system)
}

fn supported_game_file(file_name: &str) -> bool {
    let lower = file_name.to_ascii_lowercase();
    let Some(ext) = Path::new(&lower).extension().and_then(|ext| ext.to_str()) else {
        return false;
    };
    matches!(
        ext,
        "gba" | "gb" | "gbc" | "sfc" | "smc" | "nes" | "md" | "gen" | "sms" | "z64" | "n64" | "v64" | "iso" | "chd" | "cue" | "bin" | "cso" | "dol" | "gcm" | "wad" | "wbfs" | "zip" | "7z"
    )
}

fn safe_upload_name(name: &str) -> String {
    let raw = Path::new(name)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("game.bin");
    let cleaned: String = raw
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_' | ' ') { ch } else { '-' })
        .collect();
    let trimmed = cleaned.trim_matches([' ', '.', '-']).trim();
    let value = if trimmed.is_empty() { "game.bin" } else { trimmed };
    cap_upload_name(value, 128)
}

fn cap_upload_name(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let path = Path::new(value);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("game");
    let ext_len = if ext.is_empty() { 0 } else { ext.chars().count() + 1 };
    let keep = max_chars.saturating_sub(ext_len).max(1);
    let mut capped: String = stem.chars().take(keep).collect();
    if !ext.is_empty() {
        capped.push('.');
        capped.push_str(ext);
    }
    capped
}

fn public_file_name(name: &str) -> String {
    safe_upload_name(name)
}

async fn action_clear_artwork_cache(
    Json(body): Json<ConsoleActionRequest>,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    if body.confirm.as_deref() != Some("CLEAR_ARTWORK") {
        return console_action_error(
            StatusCode::BAD_REQUEST,
            "clear-artwork-cache",
            "arcadia-storage",
            "Confirm before clearing artwork cache. This does not delete games.",
        );
    }
    storage_remove_children(
        "clear-artwork-cache",
        ARTWORK_ROOT,
        "Artwork cache cleared. Games were not deleted.",
        "Artwork cache could not be cleared.",
    )
}

async fn action_clean_temporary_files(
    Json(body): Json<ConsoleActionRequest>,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    if body.confirm.as_deref() != Some("CLEAN_TEMPORARY") {
        return console_action_error(
            StatusCode::BAD_REQUEST,
            "clean-temporary-files",
            "arcadia-storage",
            "Confirm before cleaning safe temporary files.",
        );
    }
    let mut removed = 0u64;
    let mut errors = Vec::new();
    for root in TEMP_CLEAN_ROOTS {
        match remove_children(Path::new(root)) {
            Ok(count) => removed += count,
            Err(err) => errors.push(format!("{}: {}", root, err)),
        }
    }
    let ok = errors.is_empty();
    let message = if ok {
        format!("Cleaned safe temporary files. Removed {} entries. Games, artwork, and AI models were not touched.", removed)
    } else {
        format!(
            "Temporary cleanup partially failed after removing {} entries: {}",
            removed,
            errors.join("; ")
        )
    };
    (
        if ok {
            StatusCode::OK
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        },
        Json(ConsoleActionResponse {
            ok,
            action: "clean-temporary-files",
            command: "arcadia-storage",
            exit_code: if ok { Some(0) } else { Some(1) },
            message,
            stdout: String::new(),
            stderr: errors.join("\n"),
        }),
    )
}

async fn action_remove_ai_model(
    Query(params): Query<HashMap<String, String>>,
    Json(body): Json<ConsoleActionRequest>,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    if body.confirm.as_deref() != Some("REMOVE_MODEL") {
        return console_action_error(
            StatusCode::BAD_REQUEST,
            "remove-ai-model",
            "arcadia-storage",
            "Confirm before removing a local AI model file. This does not affect games.",
        );
    }
    let Some(filename) = params.get("name") else {
        return console_action_error(
            StatusCode::BAD_REQUEST,
            "remove-ai-model",
            "arcadia-storage",
            "Missing model filename.",
        );
    };
    let current = storage_status();
    let Some(path) = find_model_path_by_filename(filename) else {
        return console_action_error(
            StatusCode::NOT_FOUND,
            "remove-ai-model",
            "arcadia-storage",
            "Model file was not found in local AI storage.",
        );
    };
    let model_known = current
        .ai_models
        .models
        .iter()
        .any(|model| model.filename == *filename);
    if !model_known {
        return console_action_error(
            StatusCode::NOT_FOUND,
            "remove-ai-model",
            "arcadia-storage",
            "Model file was not found in the current storage inventory.",
        );
    }
    match fs::remove_file(&path) {
        Ok(()) => (
            StatusCode::OK,
            Json(ConsoleActionResponse {
                ok: true,
                action: "remove-ai-model",
                command: "arcadia-storage",
                exit_code: Some(0),
                message: format!(
                    "Removed {} from console storage. Games were not affected.",
                    filename
                ),
                stdout: String::new(),
                stderr: String::new(),
            }),
        ),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ConsoleActionResponse {
                ok: false,
                action: "remove-ai-model",
                command: "arcadia-storage",
                exit_code: Some(1),
                message: format!("Could not remove {}.", filename),
                stdout: String::new(),
                stderr: err.to_string(),
            }),
        ),
    }
}

async fn action_reboot_console(
    Json(body): Json<ConsoleActionRequest>,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    if body.confirm.as_deref() != Some("REBOOT") {
        return console_action_error(
            StatusCode::BAD_REQUEST,
            "reboot-console",
            SYSTEMCTL_BIN,
            "Type REBOOT to restart the console.",
        );
    }
    run_console_command(
        "reboot-console",
        SYSTEMCTL_BIN,
        &["reboot"],
        "Reboot requested.",
        "Reboot request failed.",
    )
}

async fn action_shutdown_console(
    Json(body): Json<ConsoleActionRequest>,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    if body.confirm.as_deref() != Some("SHUTDOWN") {
        return console_action_error(
            StatusCode::BAD_REQUEST,
            "shutdown-console",
            SYSTEMCTL_BIN,
            "Type SHUTDOWN to power off the console.",
        );
    }
    run_console_command(
        "shutdown-console",
        SYSTEMCTL_BIN,
        &["poweroff"],
        "Shutdown requested.",
        "Shutdown request failed.",
    )
}

async fn action_restart_gamescope(
    Json(body): Json<ConsoleActionRequest>,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    if body.confirm.as_deref() != Some("RESTART_GAMESCOPE") {
        return console_action_error(
            StatusCode::BAD_REQUEST,
            "restart-gamescope",
            SYSTEMCTL_BIN,
            "Confirm GameScope restart before closing the active game session.",
        );
    }
    run_console_command(
        "restart-gamescope",
        SYSTEMCTL_BIN,
        &["restart", "gamescope.service"],
        "GameScope restart requested.",
        "GameScope restart request failed.",
    )
}

