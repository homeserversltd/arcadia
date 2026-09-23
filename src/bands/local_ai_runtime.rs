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

const AI_NODE_PORTS: &[u16] = &[7777, 11434, 1234, 8188, 7391, 8080, 8081, 8082, 8083, 8084, 8085, 8086, 8087, 8088, 8089];

fn detect_local_ai_nodes() -> Vec<DetectedAi> {
    let self_pid = std::process::id();
    let listening_sockets = proc_listening_sockets();
    let gpu_compute_processes = nvidia_compute_processes();
    let mut nodes_by_pid = BTreeMap::new();
    let Ok(processes) = fs::read_dir("/proc") else { return Vec::new() };

    for entry in processes.flatten() {
        let Some(pid) = entry.file_name().to_str().and_then(|name| name.parse::<u32>().ok()) else { continue };
        if pid == self_pid { continue; }
        let proc_path = entry.path();
        let comm = fs::read_to_string(proc_path.join("comm")).unwrap_or_default();
        let args = fs::read(proc_path.join("cmdline")).unwrap_or_default()
            .split(|byte| *byte == 0)
            .filter(|part| !part.is_empty())
            .map(|part| String::from_utf8_lossy(part).into_owned())
            .collect::<Vec<_>>();
        let binary = ai_process_binary(comm.trim(), &args);
        let owned_inodes = proc_owned_socket_inodes(&proc_path);
        let owned_ports = owned_inodes.iter()
            .filter_map(|inode| listening_sockets.get(inode).copied())
            .collect::<BTreeSet<_>>();
        let candidate_port = owned_ports.iter().copied()
            .find(|port| AI_NODE_PORTS.contains(port))
            .or_else(|| owned_ports.iter().next().copied());
        let gpu_process_name = gpu_compute_processes.get(&pid).map(String::as_str);
        let gpu_owner = gpu_process_name.is_some() || proc_holds_rocm_kfd(&proc_path);
        let socket_owner = owned_ports.iter().any(|port| AI_NODE_PORTS.contains(port));
        if binary.is_empty() && !socket_owner && !gpu_owner { continue; }

        let (kind, label, mut detail) = if !binary.is_empty() {
            if is_ai_agent_name(binary) {
                ("agent", binary.to_string(), "AI agent".to_string())
            } else {
                let parsed = parse_resident_ai_engine(&safe_engine_parser_input(&args));
                let label = parsed.function_label;
                let detail = parsed.model_filename.unwrap_or_else(|| binary.to_string());
                ("engine", label, detail)
            }
        } else {
            let owner = gpu_process_name
                .map(safe_process_comm)
                .unwrap_or_else(|| safe_process_comm(comm.trim()));
            let source = if socket_owner { "Socket owner" } else { "GPU owner" };
            let label = format!("{source}: {owner}");
            ("engine", label.clone(), label)
        };
        if let Some(port) = candidate_port {
            detail = format!("{detail}:{port}");
        }
        nodes_by_pid.insert(pid, DetectedAi {
            kind: kind.to_string(), label, detail, port: candidate_port,
            alive: proc_path.exists(),
        });
    }
    nodes_by_pid.into_values().collect()
}

fn ai_process_binary<'a>(comm: &'a str, args: &'a [String]) -> &'a str {
    let comm_name = Path::new(comm).file_name().and_then(|name| name.to_str()).unwrap_or(comm);
    if is_ai_process_candidate(comm_name) { return comm_name; }
    for arg in args.iter().take_while(|arg| !arg.starts_with('-')) {
        let basename = Path::new(arg).file_name().and_then(|name| name.to_str()).unwrap_or(arg);
        if is_ai_process_candidate(basename) { return basename; }
    }
    ""
}

fn is_ai_process_candidate(name: &str) -> bool {
    is_ai_process_name(name) || Path::new(name).file_stem()
        .and_then(|stem| stem.to_str())
        .is_some_and(is_ai_process_name)
}

fn safe_engine_parser_input(args: &[String]) -> String {
    let mut safe_parts = Vec::new();
    for (index, arg) in args.iter().enumerate() {
        if matches!(arg.as_str(), "--reranking" | "--embedding") {
            safe_parts.push(arg.as_str());
            continue;
        }
        let model_value = arg.strip_prefix("--model=")
            .or_else(|| arg.strip_prefix("--model-path="))
            .or_else(|| arg.strip_prefix("-m="))
            .or_else(|| matches!(arg.as_str(), "--model" | "--model-path" | "-m")
                .then(|| args.get(index + 1).map(String::as_str)).flatten());
        if let Some(value) = model_value {
            let basename = Path::new(value).file_name().and_then(|name| name.to_str()).unwrap_or(value);
            let lower = basename.to_ascii_lowercase();
            if lower.ends_with(".gguf") || lower.ends_with(".safetensors") || lower.ends_with(".onnx") {
                safe_parts.push(basename);
            }
        }
    }
    safe_parts.join(" ")
}

fn safe_process_comm(comm: &str) -> String {
    let name = Path::new(comm).file_name().and_then(|name| name.to_str()).unwrap_or(comm);
    let safe = name.chars().filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.')).take(40).collect::<String>();
    if safe.is_empty() { "Unknown process".to_string() } else { safe }
}

fn is_ai_process_name(name: &str) -> bool {
    matches!(name.to_ascii_lowercase().as_str(),
        "llama-server" | "llama-cli" | "ollama" | "vllm" | "lmstudio" | "koboldcpp" |
        "text-generation" | "tabby" | "localai" | "comfyui" | "whisper" | "hermes" |
        "claude" | "codex" | "grok" | "gemini" | "aider" | "opencode" | "unio")
}

fn is_ai_agent_name(name: &str) -> bool {
    matches!(name.to_ascii_lowercase().as_str(),
        "hermes" | "claude" | "codex" | "grok" | "gemini" | "aider" | "opencode" | "unio")
}

fn proc_listening_sockets() -> HashMap<String, u16> {
    let mut sockets = HashMap::new();
    for path in ["/proc/net/tcp", "/proc/net/tcp6"] {
        let Ok(contents) = fs::read_to_string(path) else { continue };
        for line in contents.lines().skip(1) {
            let fields = line.split_whitespace().collect::<Vec<_>>();
            if fields.len() <= 9 || fields[3] != "0A" { continue; }
            let Some(port) = fields[1].rsplit(':').next().and_then(|value| u16::from_str_radix(value, 16).ok()) else { continue };
            sockets.insert(fields[9].to_string(), port);
        }
    }
    sockets
}

fn proc_owned_socket_inodes(proc_path: &Path) -> BTreeSet<String> {
    let mut inodes = BTreeSet::new();
    let Ok(fds) = fs::read_dir(proc_path.join("fd")) else { return inodes };
    for fd in fds.flatten() {
        let Ok(target) = fs::read_link(fd.path()) else { continue };
        let target = target.to_string_lossy();
        if let Some(inode) = target.strip_prefix("socket:[").and_then(|target| target.strip_suffix(']')) {
            inodes.insert(inode.to_string());
        }
    }
    inodes
}

fn nvidia_compute_processes() -> HashMap<u32, String> {
    command_stdout(
        "nvidia-smi",
        &["--query-compute-apps=pid,process_name", "--format=csv,noheader"],
    )
    .into_iter()
    .flat_map(|output| output.lines().map(str::to_string).collect::<Vec<_>>())
    .filter_map(|line| {
        let (pid, process_name) = line.split_once(',')?;
        Some((pid.trim().parse::<u32>().ok()?, process_name.trim().to_string()))
    })
    .collect()
}

fn proc_holds_rocm_kfd(proc_path: &Path) -> bool {
    let Ok(fds) = fs::read_dir(proc_path.join("fd")) else { return false };
    fds.flatten().any(|fd| {
        fs::read_link(fd.path())
            .ok()
            .is_some_and(|target| target == Path::new("/dev/kfd"))
    })
}

fn local_ai_status() -> LocalAiStatus {
    let detected_ai = detect_local_ai_nodes();
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
        detected_ai,
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
