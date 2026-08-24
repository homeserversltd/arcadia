async fn network_state_route(State(state): State<Arc<AppState>>) -> Json<NetworkState> {
    Json(state.living_snapshot().network.clone())
}

async fn ai_state_route(State(state): State<Arc<AppState>>) -> Json<LocalAIState> {
    Json(state.living_snapshot().ai.clone())
}

async fn ai_models_installed(State(state): State<Arc<AppState>>) -> Json<Vec<LocalAiModelStatus>> {
    Json(state.living_snapshot().ai.installed_models.clone())
}
async fn ai_models_recommended(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<AIRecommendedModel>> {
    Json(state.living_snapshot().ai.recommended_models.clone())
}

async fn gui_pin_status_route() -> Json<GuiPinStatus> {
    Json(gui_pin_status())
}

async fn ai_runtime_check_update(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<AIActionResponse>) {
    let Ok(value) = caduceus_post_json("/api/v1/doors", "{}") else {
        return ai_action(
            StatusCode::BAD_GATEWAY,
            &state,
            false,
            "runtime-check-update",
            "Caduceus Local AI runtime check is unreachable.",
        );
    };
    let mut cfg = load_ai_config();
    cfg.last_checked_at = Some(now_rfc3339_like());
    let _ = save_ai_config(&cfg);
    let installed = value.get("installed").and_then(|v| v.as_bool()).unwrap_or(false);
    ai_action(
        StatusCode::OK,
        &state,
        true,
        "runtime-check-update",
        if installed {
            "llama.cpp runtime is installed."
        } else {
            "llama.cpp update check completed."
        },
    )
}
async fn ai_runtime_update(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<AIActionResponse>) {
    let Ok(value) = caduceus_post_json("/api/v1/doors", "{}") else {
        return ai_action(
            StatusCode::BAD_GATEWAY,
            &state,
            false,
            "runtime-update",
            "Caduceus Local AI runtime update is unreachable.",
        );
    };
    let command_ok = value.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
    let runtime_installed = state.living_snapshot().ai.clone().runtime.installed;
    let ok = command_ok && runtime_installed;
    ai_action(
        if ok {
            StatusCode::OK
        } else {
            StatusCode::FAILED_DEPENDENCY
        },
        &state,
        ok,
        "runtime-update",
        if ok {
            "llama.cpp installed and proven through Caduceus."
        } else if command_ok {
            "Caduceus ran Harmonia, but llama.cpp is still not installed; check /var/lib/harmonia/receipts/local-ai-runtime-latest."
        } else {
            "Caduceus Local AI runtime update failed; check /var/lib/harmonia/receipts/local-ai-runtime-latest."
        },
    )
}
async fn ai_runtime_restart(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<AIActionResponse>) {
    if !state.living_snapshot().ai.clone().runtime.installed {
        return ai_action(
            StatusCode::FAILED_DEPENDENCY,
            &state,
            false,
            "runtime-restart",
            "llama.cpp is not installed. Run Update llama.cpp first.",
        );
    }
    let ok = Command::new(SYSTEMCTL_BIN)
        .args(["restart", "arcadia-llama-server.service"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
        || Command::new(SYSTEMCTL_BIN)
            .args(["restart", "llama-server.service"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
    ai_action(
        if ok {
            StatusCode::OK
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        },
        &state,
        ok,
        "runtime-restart",
        if ok {
            "Runtime restarted."
        } else {
            "Runtime restart failed or service is not installed."
        },
    )
}
async fn ai_install_recommended(
    State(state): State<Arc<AppState>>,
    Json(_body): Json<AIRecommendedInstallRequest>,
) -> (StatusCode, Json<AIActionResponse>) {
    ai_action(
        StatusCode::BAD_REQUEST,
        &state,
        false,
        "install-recommended",
        "Recommended catalog install is not configured. Use Hugging Face .gguf downloads.",
    )
}
async fn ai_hf_list_files(
    State(state): State<Arc<AppState>>,
    Json(body): Json<HFListFilesRequest>,
) -> (StatusCode, Json<HFFileListResponse>) {
    let repo = body.repo_id.trim();
    if !valid_hf_repo(repo) {
        return (
            StatusCode::BAD_REQUEST,
            Json(HFFileListResponse {
                ok: false,
                message: "Enter a Hugging Face repository like owner/model.".into(),
                files: Vec::new(),
                state: state.living_snapshot().ai.clone(),
            }),
        );
    }
    let rev = body.revision.as_deref().unwrap_or("main");
    let url = format!(
        "https://huggingface.co/api/models/{}/tree/{}?recursive=1",
        repo, rev
    );
    let text = command_stdout("curl", &["-fsSL", "--max-time", "20", &url]).unwrap_or_default();
    let mut files = Vec::new();
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
        if let Some(arr) = value.as_array() {
            for item in arr {
                let path = item
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                if !path.to_ascii_lowercase().ends_with(".gguf") {
                    continue;
                }
                let size = item.get("size").and_then(|v| v.as_u64());
                files.push(HFModelFile {
                    filename: path.to_string(),
                    size_bytes: size,
                    estimated_vram_bytes: size.map(|v| v.saturating_add(v / 5)),
                });
            }
        }
    }
    let ok = !files.is_empty();
    (
        StatusCode::OK,
        Json(HFFileListResponse {
            ok,
            message: if ok {
                "Compatible .gguf files found."
            } else {
                "No compatible .gguf files found or repository requires access."
            }
            .into(),
            files,
            state: state.living_snapshot().ai.clone(),
        }),
    )
}
async fn ai_hf_download(
    State(state): State<Arc<AppState>>,
    Json(body): Json<HFDownloadRequest>,
) -> (StatusCode, Json<AIActionResponse>) {
    if !valid_hf_repo(&body.repo_id)
        || !body.filename.to_ascii_lowercase().ends_with(".gguf")
        || body.filename.contains("..")
    {
        return ai_action(
            StatusCode::BAD_REQUEST,
            &state,
            false,
            "huggingface-download",
            "Only compatible .gguf model files can be downloaded.",
        );
    }
    let rev = body.revision.as_deref().unwrap_or("main");
    let name = Path::new(&body.filename)
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or("model.gguf");
    let _ = fs::create_dir_all(LOCAL_AI_MODEL_ROOT);
    let out = Path::new(LOCAL_AI_MODEL_ROOT).join(name);
    let url = format!(
        "https://huggingface.co/{}/resolve/{}/{}",
        body.repo_id, rev, body.filename
    );
    let ok = Command::new("curl")
        .args([
            "-fL",
            "--progress-bar",
            "-o",
            out.to_string_lossy().as_ref(),
            &url,
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    ai_action(
        if ok {
            StatusCode::OK
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        },
        &state,
        ok,
        "huggingface-download",
        if ok {
            "Model downloaded and installed."
        } else {
            "Model download failed. Check repository access, license, token requirements, or disk space."
        },
    )
}

async fn ai_model_import(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> (StatusCode, Json<AIActionResponse>) {
    let _ = fs::create_dir_all(LOCAL_AI_MODEL_ROOT);
    while let Ok(Some(field)) = multipart.next_field().await {
        let filename = field
            .file_name()
            .and_then(|v| Path::new(v).file_name().and_then(|f| f.to_str()))
            .unwrap_or("model.gguf")
            .to_string();
        if !filename.to_ascii_lowercase().ends_with(".gguf") || filename.contains("..") {
            return ai_action(
                StatusCode::BAD_REQUEST,
                &state,
                false,
                "model-import",
                "Only local .gguf model files can be imported.",
            );
        }
        let dest = Path::new(LOCAL_AI_MODEL_ROOT).join(&filename);
        let bytes = match field.bytes().await {
            Ok(bytes) => bytes,
            Err(_) => {
                return ai_action(
                    StatusCode::BAD_REQUEST,
                    &state,
                    false,
                    "model-import",
                    "Model upload failed before the console could store it.",
                )
            }
        };
        if bytes.len() < 16 || !bytes.starts_with(b"GGUF") {
            return ai_action(
                StatusCode::BAD_REQUEST,
                &state,
                false,
                "model-import",
                "Model file is not a valid GGUF file.",
            );
        }
        if fs::write(&dest, &bytes).is_ok() {
            secure_file(&dest, 0o640);
            // ArcadiaHyalosLayer forwards this event to Caduceus Hyalos; it is not written to a sidecar file.
            tracing::info!("imported {} bytes into {}", bytes.len(), dest.display());
            return ai_action(
                StatusCode::OK,
                &state,
                true,
                "model-import",
                "Model imported into Local AI storage.",
            );
        }
        return ai_action(
            StatusCode::INTERNAL_SERVER_ERROR,
            &state,
            false,
            "model-import",
            "Model import could not write to appliance storage.",
        );
    }
    ai_action(
        StatusCode::BAD_REQUEST,
        &state,
        false,
        "model-import",
        "No model file was provided.",
    )
}

async fn ai_models_rescan(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<AIActionResponse>) {
    ai_action(
        StatusCode::OK,
        &state,
        true,
        "models-rescan",
        "Model storage rescanned.",
    )
}
