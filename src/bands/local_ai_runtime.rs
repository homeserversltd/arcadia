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
        (Some(_), None) => "unknown".into(),
    }
}
fn last_ai_check_time() -> Option<String> {
    load_ai_config().last_checked_at
}
fn selected_model_id() -> Option<String> {
    load_ai_config().selected_model_id
}

fn llama_version() -> Option<String> {
    command_combined_output(LLAMA_SERVER_BIN, &["--version"])
        .or_else(|| command_combined_output(LLAMA_CPP_BIN, &["--version"]))
        .and_then(|text| text.lines().find(|line| !line.trim().is_empty()).map(|v| v.trim().to_string()))
}

fn command_combined_output(command: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(command).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let text = format!("{}{}", stdout, stderr).trim().to_string();
    if text.is_empty() { None } else { Some(text) }
}

fn recommended_ai_models(_installed: &[LocalAiModelStatus]) -> Vec<AIRecommendedModel> {
    Vec::new()
}

fn active_ai_downloads() -> Vec<AIDownloadState> {
    Vec::new()
}

#[derive(Deserialize)]
struct ModelLibraryManifestDoc {
    #[serde(default)]
    items: Vec<ModelLibraryManifestItem>,
}

#[derive(Deserialize)]
struct ModelLibraryManifestItem {
    id: String,
    #[serde(default)]
    lane: Option<String>,
    #[serde(default)]
    file: Option<String>,
    #[serde(default)]
    dest: Option<String>,
    #[serde(default)]
    repo: Option<String>,
    #[serde(default)]
    role: Option<String>,
}

#[derive(Deserialize)]
struct ModelLibraryCatalogDoc {
    #[serde(default)]
    entries: Vec<ModelLibraryCatalogEntry>,
}

#[derive(Deserialize)]
struct ModelLibraryCatalogEntry {
    id: String,
    #[serde(default)]
    lane: Option<String>,
    #[serde(default)]
    file: Option<String>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    repo: Option<String>,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    bytes: Option<u64>,
}

fn local_ai_library_models(installed: &[LocalAiModelStatus]) -> Vec<LocalAiLibraryModelStatus> {
    let mut models: HashMap<String, LocalAiLibraryModelStatus> = HashMap::new();

    for model in installed {
        if model.source == "model-library" || model.repo_id.is_some() {
            models.insert(
                model.id.clone(),
                LocalAiLibraryModelStatus {
                    id: model.id.clone(),
                    name: model.name.clone(),
                    lane: lane_from_model_path_or_id(None, &model.id),
                    artifact: model.filename.clone(),
                    source: model.source.clone(),
                    repo_id: model.repo_id.clone(),
                    status: "available".to_string(),
                    size_bytes: Some(model.size_bytes),
                    size: model.size.clone(),
                    path: None,
                    role: model.recommended_use.map(str::to_string),
                },
            );
        }
    }

    if let Ok(text) = fs::read_to_string(MODEL_LIBRARY_MANIFEST_PATH) {
        if let Ok(doc) = serde_json::from_str::<ModelLibraryManifestDoc>(&text) {
            for item in doc.items {
                let artifact = item.file.clone().or(item.dest.clone()).unwrap_or_else(|| item.id.clone());
                if !library_model_artifact(&artifact) {
                    continue;
                }
                let path = item.dest.as_ref().map(|dest| {
                    Path::new(MODEL_LIBRARY_ROOT)
                        .join(dest)
                        .to_string_lossy()
                        .to_string()
                });
                let status = path
                    .as_ref()
                    .filter(|p| Path::new(p).exists())
                    .map(|_| "available")
                    .unwrap_or("declared")
                    .to_string();
                models.entry(item.id.clone()).or_insert_with(|| LocalAiLibraryModelStatus {
                    id: item.id.clone(),
                    name: friendly_model_name(&artifact),
                    lane: item.lane.clone().unwrap_or_else(|| lane_from_model_path_or_id(item.dest.as_deref(), &item.id)),
                    artifact,
                    source: "model-library manifest".to_string(),
                    repo_id: item.repo.clone(),
                    status,
                    size_bytes: None,
                    size: "Unknown".to_string(),
                    path,
                    role: item.role.clone(),
                });
            }
        }
    }

    if let Ok(text) = fs::read_to_string(MODEL_LIBRARY_CATALOG_PATH) {
        if let Ok(doc) = serde_json::from_str::<ModelLibraryCatalogDoc>(&text) {
            for entry in doc.entries {
                let artifact = entry
                    .file
                    .clone()
                    .or_else(|| entry.path.as_ref().and_then(|p| Path::new(p).file_name().and_then(|f| f.to_str()).map(str::to_string)))
                    .unwrap_or_else(|| entry.id.clone());
                if !library_model_artifact(&artifact) {
                    continue;
                }
                models.insert(
                    entry.id.clone(),
                    LocalAiLibraryModelStatus {
                        id: entry.id.clone(),
                        name: friendly_model_name(&artifact),
                        lane: entry.lane.clone().unwrap_or_else(|| lane_from_model_path_or_id(entry.path.as_deref(), &entry.id)),
                        artifact,
                        source: "model-library catalog".to_string(),
                        repo_id: entry.repo.clone(),
                        status: entry.status.clone().unwrap_or_else(|| "available".to_string()),
                        size_bytes: entry.bytes,
                        size: entry.bytes.map(human_size).unwrap_or_else(|| "Unknown".to_string()),
                        path: entry.path.clone(),
                        role: entry.role.clone(),
                    },
                );
            }
        }
    }

    let mut out = models.into_values().collect::<Vec<_>>();
    out.sort_by(|a, b| a.lane.cmp(&b.lane).then_with(|| a.name.cmp(&b.name)));
    out
}

fn library_model_artifact(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".gguf")
        || lower.ends_with(".onnx")
        || lower.ends_with(".safetensors")
        || lower.ends_with("model.safetensors.index.json")
}

fn lane_from_model_path_or_id(path: Option<&str>, id: &str) -> String {
    path.and_then(|p| p.split("models/").nth(1))
        .and_then(|rest| rest.split('/').next())
        .filter(|lane| !lane.is_empty())
        .map(str::to_string)
        .or_else(|| id.split('.').next().filter(|part| *part != id).map(str::to_string))
        .unwrap_or_else(|| "library".to_string())
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
    let text = path.to_string_lossy();
    if text.contains(MODEL_LIBRARY_ROOT) {
        "model-library".to_string()
    } else if text.contains("huggingface") {
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
    let resident_engines = command_stdout("pgrep", &["-af", "llama|ollama|vllm"])
        .unwrap_or_default()
        .lines()
        .map(parse_resident_ai_engine)
        .collect::<Vec<_>>();
    let loaded_model = resident_engines.first().map(|engine| {
        engine
            .model_filename
            .clone()
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
    let library_models = local_ai_library_models(&available_models);
    let cfg = load_ai_config();
    let lan_inference_enabled = cfg.lan_enabled && tcp_port_listening(cfg.lan_port);
    LocalAiStatus {
        load_state,
        parallel_slots: cfg.concurrency,
        selected_model_id: selected.map(|model| model.id.clone()),
        selected_model_name: selected.map(|model| model.name.clone()),
        loaded_model_id: loaded_model.as_ref().map(|name| model_id(name)),
        loaded_model_name: loaded_model.as_ref().map(|name| friendly_model_name(name)),
        loaded_model,
        resident_engines,
        available_models,
        library_models,
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

fn parse_resident_ai_engine(line: &str) -> ResidentAiEngineStatus {
    let function_label = if line.contains("--reranking") { "Search ranking" } else if line.contains("--embedding") { "Text understanding" } else { "AI model" };
    let model_filename = line.split_whitespace().find_map(|part| {
        let candidate = part.trim_matches(|ch| ch == '"' || ch == '\'').split_once('=').map(|(_, value)| value).unwrap_or(part).trim_matches(|ch| ch == '"' || ch == '\'');
        let lower = candidate.to_ascii_lowercase();
        if lower.ends_with(".gguf") || lower.ends_with(".safetensors") || lower.ends_with(".onnx") { Path::new(candidate).file_name().and_then(|name| name.to_str()).map(str::to_string) } else { None }
    });
    ResidentAiEngineStatus { function_label: function_label.to_string(), model_filename }
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
