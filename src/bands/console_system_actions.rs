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
    run_console_command(
        "check-updates",
        HARMONIA_BIN,
        &[
            "homeconsole-update",
            HOMECONSOLE_PROFILE,
            "--receipt-dir",
            "/var/lib/harmonia/receipts/homeconsole-check-latest",
        ],
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
    let text = fs::read_to_string(HARMONIA_HOMECONSOLE_LEDGER).unwrap_or_default();
    let mut parsed = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(mut value) = serde_json::from_str::<serde_json::Value>(trimmed) {
            redact_json_value(&mut value);
            parsed.push(harmonia_ledger_entry(idx + 1, value));
        }
    }
    parsed.reverse();
    let total_entries = parsed.len();
    let total_pages = total_entries.div_ceil(per_page).max(1);
    let bounded_page = page.min(total_pages);
    let start = (bounded_page - 1) * per_page;
    let entries = parsed.into_iter().skip(start).take(per_page).collect::<Vec<_>>();
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
    run_console_command(
        "sync-games",
        HARMONIA_BIN,
        &[
            "homeconsole-sync",
            HOMECONSOLE_PROFILE,
            "--provider-env",
            PROVIDER_KEYS_PATH,
            "--adapter-command",
            ARCH_GAME_SYNC_BIN,
            "--apply",
            "--receipt-dir",
            "/var/lib/harmonia/receipts/game-sync-latest",
        ],
        "Sync games completed.",
        "Sync games failed. Read /var/lib/harmonia/receipts/game-sync-latest.",
    )
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

