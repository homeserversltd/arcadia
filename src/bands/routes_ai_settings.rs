async fn ai_settings_save(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AISettingsRequest>,
) -> (StatusCode, Json<AIActionResponse>) {
    let mut cfg = load_ai_config();
    if let Some(v) = body.start_api_on_boot {
        cfg.start_api_on_boot = v;
    }
    if let Some(v) = body.auto_load_last_model {
        cfg.auto_load_last_model = v;
    }
    if let Some(v) = body.preferred_model_id {
        cfg.preferred_model_id = if v.trim().is_empty() { None } else { Some(v) };
    }
    if let Some(v) = body.context_size {
        if !(512..=262144).contains(&v) {
            return ai_action(
                StatusCode::BAD_REQUEST,
                &state,
                false,
                "settings-save",
                "Context size is outside the safe supported range.",
            );
        }
        cfg.context_size = v;
    }
    if let Some(v) = body.gpu_layers {
        if !(-1..=999).contains(&v) {
            return ai_action(
                StatusCode::BAD_REQUEST,
                &state,
                false,
                "settings-save",
                "Accelerator layers must be -1 or a non-negative supported count.",
            );
        }
        cfg.gpu_layers = v;
    }
    if let Some(v) = body.threads {
        if v > 256 {
            return ai_action(
                StatusCode::BAD_REQUEST,
                &state,
                false,
                "settings-save",
                "Thread count is outside the safe supported range.",
            );
        }
        cfg.threads = v;
    }
    if let Some(v) = body.batch {
        if !(1..=8192).contains(&v) {
            return ai_action(
                StatusCode::BAD_REQUEST,
                &state,
                false,
                "settings-save",
                "Batch size is outside the safe supported range.",
            );
        }
        cfg.batch = v;
    }
    if let Some(v) = body.concurrency {
        if !(1..=64).contains(&v) {
            return ai_action(
                StatusCode::BAD_REQUEST,
                &state,
                false,
                "settings-save",
                "Concurrency is outside the safe supported range.",
            );
        }
        cfg.concurrency = v;
    }
    if let Some(v) = body.request_limit {
        if !(1..=10000).contains(&v) {
            return ai_action(
                StatusCode::BAD_REQUEST,
                &state,
                false,
                "settings-save",
                "Request limit is outside the safe supported range.",
            );
        }
        cfg.request_limit = v;
    }
    if let Some(v) = body.lan_port {
        if !valid_lan_port(v) {
            return ai_action(
                StatusCode::BAD_REQUEST,
                &state,
                false,
                "settings-save",
                "LAN port must be 1024-65535 and cannot conflict with HomeConsole service ports.",
            );
        }
        cfg.lan_port = v;
    }
    if let Some(v) = body.cors_origins {
        cfg.cors_origins = v
            .into_iter()
            .filter(|o| o.starts_with("http://") || o.starts_with("https://"))
            .collect();
    }
    if let Some(v) = body.log_level {
        if !matches!(v.as_str(), "error" | "warn" | "info" | "debug" | "trace") {
            return ai_action(
                StatusCode::BAD_REQUEST,
                &state,
                false,
                "settings-save",
                "Log level must be error, warn, info, debug, or trace.",
            );
        }
        cfg.log_level = v;
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
        "settings-save",
        if ok {
            "Local AI settings saved."
        } else {
            "Local AI settings could not be saved."
        },
    )
}

fn ai_action(
    status: StatusCode,
    state: &AppState,
    ok: bool,
    action: &'static str,
    message: &str,
) -> (StatusCode, Json<AIActionResponse>) {
    if ok {
        state.request_living_refresh();
    }
    (
        status,
        Json(AIActionResponse {
            ok,
            action,
            message: message.to_string(),
            state: state.living_snapshot().ai.clone(),
        }),
    )
}
fn valid_hf_repo(repo: &str) -> bool {
    repo.split('/').count() == 2
        && !repo.contains("..")
        && repo
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '-' | '_' | '.'))
}
