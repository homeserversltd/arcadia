async fn ai_download_cancel(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<AIActionResponse>) {
    ai_action(
        StatusCode::OK,
        &state,
        true,
        "download-cancel",
        "No active download is running.",
    )
}
async fn ai_model_remove(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AIModelIdRequest>,
) -> (StatusCode, Json<AIActionResponse>) {
    if body.confirm.as_deref() != Some("REMOVE_MODEL") {
        return ai_action(
            StatusCode::BAD_REQUEST,
            &state,
            false,
            "model-remove",
            "Remove this model from console storage? Games and artwork are not affected.",
        );
    }
    let filename = body
        .filename
        .or_else(|| {
            body.model_id.and_then(|id| {
                local_ai_available_models()
                    .into_iter()
                    .find(|m| m.id == id)
                    .map(|m| m.filename)
            })
        })
        .unwrap_or_default();
    let loaded = local_ai_status().loaded_model.unwrap_or_default();
    if loaded == filename {
        return ai_action(
            StatusCode::BAD_REQUEST,
            &state,
            false,
            "model-remove",
            "Unload before removing this model.",
        );
    }
    match find_model_path_by_filename(&filename).and_then(|p| fs::remove_file(p).ok().map(|_| ())) {
        Some(_) => ai_action(
            StatusCode::OK,
            &state,
            true,
            "model-remove",
            "Model removed from console storage.",
        ),
        None => ai_action(
            StatusCode::NOT_FOUND,
            &state,
            false,
            "model-remove",
            "Model file was not found.",
        ),
    }
}
async fn ai_model_select(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AIModelIdRequest>,
) -> (StatusCode, Json<AIActionResponse>) {
    let id = body.model_id.unwrap_or_default();
    if local_ai_available_models().iter().any(|m| m.id == id) {
        let _ = fs::create_dir_all(
            Path::new(LOCAL_AI_STATE_PATH)
                .parent()
                .unwrap_or(Path::new("/var/lib/arcadia")),
        );
        let mut cfg = load_ai_config();
        cfg.selected_model_id = Some(id.clone());
        let _ = save_ai_config(&cfg);
        ai_action(
            StatusCode::OK,
            &state,
            true,
            "model-select",
            "Model selected.",
        )
    } else {
        ai_action(
            StatusCode::NOT_FOUND,
            &state,
            false,
            "model-select",
            "Selected model is not installed.",
        )
    }
}
async fn ai_model_load(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AIModelIdRequest>,
) -> (StatusCode, Json<AIActionResponse>) {
    let id = body.model_id.unwrap_or_else(|| {
        local_ai_state(&state)
            .loaded_model
            .selected_model_id
            .unwrap_or_default()
    });
    let Some(model) = local_ai_available_models().into_iter().find(|m| m.id == id) else {
        return ai_action(
            StatusCode::NOT_FOUND,
            &state,
            false,
            "model-load",
            "No installed model is selected.",
        );
    };
    let Some(path) = find_model_path_by_filename(&model.filename) else {
        return ai_action(
            StatusCode::NOT_FOUND,
            &state,
            false,
            "model-load",
            "Model file was not found.",
        );
    };
    let cfg = load_ai_config();
    let bind_host = if cfg.lan_enabled { "0.0.0.0" } else { "127.0.0.1" };
    let ok = helper_exists(LLAMA_SERVER_BIN)
        && Command::new(SYSTEMD_RUN_BIN)
            .args([
                "--unit=arcadia-llama-server",
                "--collect",
                "--property=Restart=on-failure",
                "--property=RestartSec=2",
                LLAMA_SERVER_BIN,
                "-m",
                path.to_string_lossy().as_ref(),
                "--port",
                &cfg.lan_port.to_string(),
                "--host",
                bind_host,
            ])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
    ai_action(
        if ok {
            StatusCode::OK
        } else {
            StatusCode::FAILED_DEPENDENCY
        },
        &state,
        ok,
        "model-load",
        if ok {
            "Model load started."
        } else {
            "llama.cpp server is missing or the model could not be started."
        },
    )
}
async fn ai_model_unload(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<AIActionResponse>) {
    let ok = Command::new("pkill")
        .args(["-f", "llama-server"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    ai_action(
        StatusCode::OK,
        &state,
        true,
        "model-unload",
        if ok {
            "Model unloaded."
        } else {
            "No loaded model was running."
        },
    )
}
async fn ai_inference_set_enabled(
    State(state): State<Arc<AppState>>,
    Json(body): Json<InferenceSetRequest>,
) -> (StatusCode, Json<AIActionResponse>) {
    if body.enabled {
        let ai = local_ai_state(&state);
        if !ai.runtime.installed {
            return ai_action(
                StatusCode::FAILED_DEPENDENCY,
                &state,
                false,
                "inference-set-enabled",
                "llama.cpp is not installed. Run Update llama.cpp before enabling inference.",
            );
        }
        if ai.installed_models.is_empty() {
            return ai_action(
                StatusCode::FAILED_DEPENDENCY,
                &state,
                false,
                "inference-set-enabled",
                "No GGUF model is installed. Import or download a model before enabling inference.",
            );
        }
    }
    let mut cfg = load_ai_config();
    cfg.api_enabled = body.enabled;
    if !body.enabled {
        cfg.lan_enabled = false;
        let _ = stop_llama_server();
        let _ = apply_lan_exposure(&cfg);
    }
    let ok = save_ai_config(&cfg).is_ok();
    ai_action(
        if ok {
            StatusCode::OK
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        },
        &state,
        ok,
        "inference-set-enabled",
        if body.enabled {
            "Local inference API enabled in internal-only mode until LAN access is explicitly enabled."
        } else {
            "Local inference API disabled and LAN exposure removed."
        },
    )
}
async fn ai_inference_set_lan_access(
    State(state): State<Arc<AppState>>,
    Json(body): Json<InferenceLanRequest>,
) -> (StatusCode, Json<AIActionResponse>) {
    if body.enabled {
        let ai = local_ai_state(&state);
        if !ai.runtime.installed {
            return ai_action(
                StatusCode::FAILED_DEPENDENCY,
                &state,
                false,
                "inference-set-lan-access",
                "llama.cpp is not installed. Run Update llama.cpp before exposing Local AI on LAN.",
            );
        }
        let cfg_now = load_ai_config();
        if !tcp_port_listening(cfg_now.lan_port) && !tcp_port_listening(DEFAULT_LAN_INFERENCE_PORT) {
            return ai_action(
                StatusCode::FAILED_DEPENDENCY,
                &state,
                false,
                "inference-set-lan-access",
                "Local inference server is not running. Load a model before exposing Local AI on LAN.",
            );
        }
    }
    let mut cfg = load_ai_config();
    if let Some(port) = body.port {
        if !valid_lan_port(port) {
            return ai_action(
                StatusCode::BAD_REQUEST,
                &state,
                false,
                "inference-set-lan-access",
                "LAN port must be 1024-65535 and cannot conflict with HomeConsole service ports.",
            );
        }
        cfg.lan_port = port;
    }
    if let Some(cidr) = body.lan_cidr.as_deref() {
        if !valid_lan_cidr(cidr) {
            return ai_action(
                StatusCode::BAD_REQUEST,
                &state,
                false,
                "inference-set-lan-access",
                "LAN CIDR must be a private IPv4 CIDR such as 10.0.0.0/24.",
            );
        }
        cfg.lan_cidr = cidr.to_string();
    }
    cfg.lan_enabled = body.enabled;
    let apply = apply_lan_exposure(&cfg);
    let ok = apply.is_ok() && save_ai_config(&cfg).is_ok();
    ai_action(
        if ok {
            StatusCode::OK
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        },
        &state,
        ok,
        "inference-set-lan-access",
        if ok && body.enabled {
            "LAN access applied through the trusted-home-LAN port."
        } else if ok {
            "LAN access disabled and proxy/firewall state removed where possible."
        } else {
            "LAN access could not be applied; previous config remains active."
        },
    )
}
async fn ai_inference_test(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<AIActionResponse>) {
    let cfg = load_ai_config();
    let ok = tcp_port_listening(cfg.lan_port) || tcp_port_listening(DEFAULT_LAN_INFERENCE_PORT);
    ai_action(
        if ok {
            StatusCode::OK
        } else {
            StatusCode::FAILED_DEPENDENCY
        },
        &state,
        ok,
        "inference-test",
        if ok {
            "Inference endpoint is listening."
        } else {
            "No model is serving inference on the configured Local AI port."
        },
    )
}

