fn stop_llama_server() -> bool {
    Command::new(SYSTEMCTL_BIN)
        .args(["stop", "arcadia-llama-server.service"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
        || Command::new("pkill")
            .args(["-f", "llama-server"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
}

fn llama_binary_path() -> Option<String> {
    [
        LLAMA_SERVER_BIN,
        LLAMA_CPP_BIN,
        "/usr/bin/llama-server",
        "/usr/bin/llama-cli",
    ]
    .iter()
    .find(|p| Path::new(p).exists())
    .map(|p| p.to_string())
}
fn llama_source_path() -> Option<String> {
    [
        "/opt/llama.cpp",
        "/opt/llama-cpp/source",
        "/usr/local/src/llama.cpp",
    ]
    .iter()
    .find(|p| Path::new(p).exists())
    .map(|p| p.to_string())
}
fn latest_llama_version() -> Option<String> {
    fs::read_to_string("/var/lib/harmonia/state/llama-cpp-latest.version")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}
fn runtime_update_state() -> String {
    let current = llama_version();
    let latest = latest_llama_version();
    match (current, latest) {
        (None, _) => "missing".into(),
        (Some(c), Some(l)) if c != l => "available".into(),
        (Some(_), Some(_)) => "current".into(),
        _ => "unknown".into(),
    }
}
fn last_ai_check_time() -> Option<String> {
    load_ai_config().last_checked_at
}
fn selected_model_id() -> Option<String> {
    load_ai_config().selected_model_id
}

fn llama_version() -> Option<String> {
    command_stdout(LLAMA_SERVER_BIN, &["--version"])
        .or_else(|| command_stdout(LLAMA_CPP_BIN, &["--version"]))
        .and_then(|text| text.lines().next().map(|v| v.trim().to_string()))
}

fn recommended_ai_models(_installed: &[LocalAiModelStatus]) -> Vec<AIRecommendedModel> {
    Vec::new()
}

fn active_ai_downloads() -> Vec<AIDownloadState> {
    Vec::new()
}

fn redacted_log(path: &str) -> String {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .rev()
        .take(40)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n")
        .replace("token", "[REDACTED]")
        .replace("password", "[REDACTED]")
        .replace("api_key", "[REDACTED]")
}

fn quantization_from_filename(filename: &str) -> Option<String> {
    let upper = filename.to_ascii_uppercase();
    [
        "Q2_K",
        "Q3_K_M",
        "Q4_K_M",
        "Q5_K_M",
        "Q6_K",
        "Q8_0",
        "UD_Q3_K_M",
    ]
    .iter()
    .find(|q| upper.contains(**q))
    .map(|q| q.to_string())
}

fn model_source_from_path(_filename: &str, path: &Path) -> String {
    if path.to_string_lossy().contains("huggingface") {
        "huggingface".to_string()
    } else {
        "manual".to_string()
    }
}

fn repo_id_from_path(path: &Path) -> Option<String> {
    let text = path.to_string_lossy();
    let marker = "models--";
    let start = text.find(marker)? + marker.len();
    let rest = &text[start..];
    let repo = rest.split('/').next()?.replace("--", "/");
    Some(repo)
}

fn local_ai_status() -> LocalAiStatus {
    let mut available_models = local_ai_available_models();
    let loaded_model = command_stdout("pgrep", &["-af", "llama|ollama|vllm"])
        .and_then(|text| text.lines().next().map(str::to_string))
        .map(|line| {
            line.split_whitespace()
                .find(|part| {
                    part.ends_with(".gguf")
                        || part.ends_with(".safetensors")
                        || part.ends_with(".onnx")
                })
                .map(|part| {
                    Path::new(part)
                        .file_name()
                        .and_then(|v| v.to_str())
                        .unwrap_or(part)
                        .to_string()
                })
                .unwrap_or_else(|| "Local AI runtime".to_string())
        });
    if let Some(loaded) = &loaded_model {
        if !available_models
            .iter()
            .any(|model| model.filename == *loaded)
        {
            available_models.insert(
                0,
                LocalAiModelStatus {
                    id: model_id(loaded),
                    name: friendly_model_name(loaded),
                    filename: loaded.clone(),
                    source: "manual".to_string(),
                    repo_id: None,
                    size_bytes: 0,
                    size: "Unknown".to_string(),
                    quantization: quantization_from_filename(loaded),
                    estimated_vram_bytes: None,
                    recommended_use: None,
                    installed_at: None,
                    is_recommended: false,
                },
            );
        }
    }
    let selected = loaded_model
        .as_ref()
        .and_then(|loaded| {
            available_models
                .iter()
                .find(|model| model.filename == *loaded)
        })
        .or_else(|| available_models.first());
    let load_state = if loaded_model.is_some() {
        "hot"
    } else if selected.is_some() {
        "cold"
    } else {
        "unloaded"
    }
    .to_string();
    let (gpu_used, gpu_total) = gpu_memory_bytes();
    let cfg = load_ai_config();
    let lan_inference_enabled = cfg.lan_enabled && tcp_port_listening(cfg.lan_port);
    LocalAiStatus {
        load_state,
        selected_model_id: selected.map(|model| model.id.clone()),
        selected_model_name: selected.map(|model| model.name.clone()),
        loaded_model_id: loaded_model.as_ref().map(|name| model_id(name)),
        loaded_model_name: loaded_model.as_ref().map(|name| friendly_model_name(name)),
        loaded_model,
        available_models,
        gpu_memory: match (gpu_used, gpu_total) {
            (Some(used), Some(total)) => {
                Some(format!("{} / {}", human_size(used), human_size(total)))
            }
            _ => None,
        },
        gpu_memory_used_bytes: gpu_used,
        gpu_memory_total_bytes: gpu_total,
        lan_inference_enabled,
        lan_inference_port: lan_inference_enabled.then_some(cfg.lan_port),
    }
}

fn local_ai_available_models() -> Vec<LocalAiModelStatus> {
    let mut models = Vec::new();
    for root in MODEL_SCAN_ROOTS {
        let mut found = Vec::new();
        collect_ai_models(Path::new(root), &mut found, 0);
        for (size, filename, _path) in found {
            if !filename.to_ascii_lowercase().ends_with(".gguf") {
                continue;
            }
            models.push(LocalAiModelStatus {
                id: model_id(&filename),
                name: friendly_model_name(&filename),
                filename: filename.clone(),
                source: model_source_from_path(&filename, &_path),
                repo_id: repo_id_from_path(&_path),
                size_bytes: size,
                size: human_size(size),
                quantization: quantization_from_filename(&filename),
                estimated_vram_bytes: Some(size.saturating_add(size / 5)),
                recommended_use: Some(if size < 3_000_000_000 {
                    "fast"
                } else if size < 6_000_000_000 {
                    "balanced"
                } else {
                    "quality"
                }),
                installed_at: None,
                is_recommended: false,
            });
        }
    }
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

fn model_id(filename: &str) -> String {
    filename
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn gpu_memory_bytes() -> (Option<u64>, Option<u64>) {
    let Some(text) = command_stdout(
        "nvidia-smi",
        &[
            "--query-gpu=memory.used,memory.total",
            "--format=csv,noheader,nounits",
        ],
    ) else {
        return (None, None);
    };
    let first = text.lines().next().unwrap_or_default();
    let mut parts = first
        .split(',')
        .map(|part| part.trim().parse::<u64>().ok().map(|mib| mib * 1024 * 1024));
    (parts.next().flatten(), parts.next().flatten())
}
