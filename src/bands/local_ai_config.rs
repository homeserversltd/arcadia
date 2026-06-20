fn local_ai_state(state: &AppState) -> LocalAIState {
    let status = local_ai_status();
    let installed = status.available_models.clone();
    let runtime_installed = helper_exists(LLAMA_SERVER_BIN)
        || helper_exists(LLAMA_CPP_BIN)
        || command_stdout("which", &["llama-server"]).is_some();
    let cfg = load_ai_config();
    let inference_listening =
        tcp_port_listening(cfg.lan_port) || tcp_port_listening(DEFAULT_LAN_INFERENCE_PORT);
    let server_running =
        inference_listening || command_stdout("pgrep", &["-af", "llama-server"]).is_some();
    let storage = storage_status();
    let free_storage = storage.free_bytes;
    let model_storage = storage.ai_models.bytes;
    let endpoint = format!(
        "{}:{}",
        state.canonical_url.trim_end_matches('/'),
        cfg.lan_port
    );
    let selected = selected_model_id().or_else(|| status.selected_model_id.clone());
    let selected_name = selected
        .as_ref()
        .and_then(|id| {
            installed
                .iter()
                .find(|m| &m.id == id)
                .map(|m| m.name.clone())
        })
        .or(status.selected_model_name.clone());
    LocalAIState {
        runtime: AIRuntimeState {
            installed: runtime_installed,
            name: "llama.cpp".to_string(),
            version: llama_version(),
            latest_version: latest_llama_version(),
            update_state: runtime_update_state(),
            server_state: if server_running { "running" } else { "stopped" }.to_string(),
            binary_path: llama_binary_path(),
            source_path: llama_source_path(),
            description: "llama.cpp runs local GGUF models and exposes an OpenAI-compatible inference API on this appliance.".to_string(),
            last_checked_at: last_ai_check_time(),
            error: None,
        },
        loaded_model: AILoadedModelState {
            load_state: status.load_state.clone(),
            selected_model_id: selected,
            selected_model_name: selected_name,
            loaded_model_id: status.loaded_model_id.clone(),
            loaded_model_name: status.loaded_model_name.clone(),
            error: None,
        },
        installed_models: installed.clone(),
        recommended_models: recommended_ai_models(&installed),
        downloads: active_ai_downloads(),
        inference: InferenceState {
            enabled: inference_listening,
            lan_access_enabled: cfg.lan_enabled && inference_listening,
            access_mode: if !cfg.api_enabled { "off" } else if cfg.lan_enabled { "lan" } else { "internal-only" }.to_string(),
            host: if cfg.lan_enabled && inference_listening {
                "lan"
            } else {
                "127.0.0.1"
            }
            .to_string(),
            port: cfg.lan_port,
            endpoint_urls: if inference_listening {
                vec![endpoint.clone()]
            } else {
                Vec::new()
            },
            api_mode: Some("openai-compatible".to_string()),
            request_count: None,
            last_request_at: None,
            nginx_configured: Path::new(LOCAL_AI_NGINX_CONF).exists(),
            firewall_configured: Path::new(LOCAL_AI_FIREWALL_RECEIPT).exists(),
            health_path: "/health".to_string(),
        },
        hardware: AIHardwareState {
            gpu_memory_used_bytes: status.gpu_memory_used_bytes,
            gpu_memory_total_bytes: status.gpu_memory_total_bytes,
            model_storage_bytes: model_storage,
            free_storage_bytes: free_storage,
        },
        activity: AIActivityState {
            current_operation: if inference_listening {
                "serving inference"
            } else if server_running {
                "runtime running"
            } else {
                "idle"
            }
            .to_string(),
            last_error: None,
            runtime_update_log: redacted_log(
                "/var/lib/harmonia/receipts/local-ai-runtime-latest/events.jsonl",
            ),
            model_download_log: redacted_log("/var/lib/arcadia/local-ai-download.log"),
            model_load_log: redacted_log("/var/log/arcadia-local-ai.log"),
            inference_server_log: redacted_log("/var/log/llama-server.log"),
        },
        settings: ai_settings_state(&cfg),
        client_handoff: ai_client_handoff(&cfg, inference_listening, endpoint),
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct LocalAiConfig {
    api_enabled: bool,
    lan_enabled: bool,
    lan_port: u16,
    lan_cidr: String,
    selected_model_id: Option<String>,
    start_api_on_boot: bool,
    auto_load_last_model: bool,
    preferred_model_id: Option<String>,
    context_size: u32,
    gpu_layers: i32,
    threads: u32,
    batch: u32,
    concurrency: u32,
    request_limit: u32,
    cors_origins: Vec<String>,
    log_level: String,
    last_checked_at: Option<String>,
}

impl Default for LocalAiConfig {
    fn default() -> Self {
        Self {
            api_enabled: false,
            lan_enabled: false,
            lan_port: DEFAULT_LAN_INFERENCE_PORT,
            lan_cidr: "192.168.123.0/24".to_string(),
            selected_model_id: None,
            start_api_on_boot: false,
            auto_load_last_model: false,
            preferred_model_id: None,
            context_size: 4096,
            gpu_layers: -1,
            threads: 0,
            batch: 512,
            concurrency: 1,
            request_limit: 60,
            cors_origins: vec![
                "http://console.home.arpa".to_string(),
                "http://homeconsole.home.arpa".to_string(),
            ],
            log_level: "info".to_string(),
            last_checked_at: None,
        }
    }
}

fn load_ai_config() -> LocalAiConfig {
    fs::read_to_string(LOCAL_AI_STATE_PATH)
        .ok()
        .and_then(|text| serde_json::from_str::<LocalAiConfig>(&text).ok())
        .unwrap_or_default()
}

fn save_ai_config(cfg: &LocalAiConfig) -> std::io::Result<()> {
    let path = Path::new(LOCAL_AI_STATE_PATH);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(cfg).unwrap_or_else(|_| "{}".to_string());
    fs::write(path, text)?;
    secure_file(path, 0o640);
    Ok(())
}

fn ai_settings_state(cfg: &LocalAiConfig) -> AISettingsState {
    AISettingsState {
        start_api_on_boot: cfg.start_api_on_boot,
        auto_load_last_model: cfg.auto_load_last_model,
        preferred_model_id: cfg.preferred_model_id.clone(),
        context_size: cfg.context_size,
        gpu_layers: cfg.gpu_layers,
        threads: cfg.threads,
        batch: cfg.batch,
        concurrency: cfg.concurrency,
        request_limit: cfg.request_limit,
        lan_cidr: cfg.lan_cidr.clone(),
        cors_origins: cfg.cors_origins.clone(),
        log_level: cfg.log_level.clone(),
    }
}

fn ai_client_handoff(
    cfg: &LocalAiConfig,
    listening: bool,
    endpoint: String,
) -> AIClientHandoffState {
    let token = fs::read_to_string(LOCAL_AI_TOKEN_PATH)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    AIClientHandoffState {
        endpoint: (cfg.api_enabled && listening).then(|| endpoint.clone()),
        openai_base_url: (cfg.api_enabled && listening).then(|| format!("{}/v1", endpoint.trim_end_matches('/'))),
        token_configured: token.is_some(),
        token_preview: token.as_ref().map(|t| format!("{}…{}", &t[..t.len().min(4)], &t[t.len().saturating_sub(4)..])),
        hermes_hint: "Set local provider base URL to the redacted OpenAI-compatible endpoint shown here; keep token material in Hermes env/config, not logs.".to_string(),
        pi_hint: "Use the LAN endpoint only from trusted home LAN clients; internal-only mode is console-local.".to_string(),
        secret_values_recorded: false,
    }
}

fn valid_lan_port(port: u16) -> bool {
    (1024..=65535).contains(&port) && !matches!(port, 22 | 80 | 443 | 445 | 8080)
}

fn valid_lan_cidr(cidr: &str) -> bool {
    let Some((addr, prefix)) = cidr.split_once('/') else {
        return false;
    };
    let Ok(prefix) = prefix.parse::<u8>() else {
        return false;
    };
    if !(8..=32).contains(&prefix) {
        return false;
    }
    let Ok(ip) = addr.parse::<Ipv4Addr>() else {
        return false;
    };
    ip.is_private()
}

fn secure_file(path: &Path, mode: u32) {
    #[cfg(unix)]
    if let Ok(meta) = fs::metadata(path) {
        let mut perms = meta.permissions();
        perms.set_mode(mode);
        let _ = fs::set_permissions(path, perms);
    }
}

fn append_local_ai_log(line: &str) {
    let _ = fs::create_dir_all("/var/log");
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("/var/log/arcadia-local-ai.log")
    {
        let _ = writeln!(file, "{} {}", now_rfc3339_like(), line);
    }
}

fn apply_lan_exposure(cfg: &LocalAiConfig) -> Result<(), String> {
    if !cfg.lan_enabled {
        let _ = fs::remove_file(LOCAL_AI_NGINX_CONF);
        let _ = fs::remove_file(LOCAL_AI_FIREWALL_RECEIPT);
        let _ = Command::new(SYSTEMCTL_BIN)
            .args(["reload", "nginx"])
            .status();
        return Ok(());
    }
    if !valid_lan_port(cfg.lan_port) || !valid_lan_cidr(&cfg.lan_cidr) {
        return Err("invalid LAN exposure config".into());
    }
    let conf = format!("server {{\n    listen {};\n    allow {};\n    deny all;\n    location / {{ proxy_pass http://127.0.0.1:{}; proxy_http_version 1.1; proxy_set_header Host $host; }}\n}}\n", cfg.lan_port, cfg.lan_cidr, cfg.lan_port);
    if let Some(parent) = Path::new(LOCAL_AI_NGINX_CONF).parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let prev = fs::read_to_string(LOCAL_AI_NGINX_CONF).ok();
    fs::write(LOCAL_AI_NGINX_CONF, conf).map_err(|e| e.to_string())?;
    let nginx_ok = Command::new("nginx")
        .arg("-t")
        .status()
        .map(|s| s.success())
        .unwrap_or(true);
    if !nginx_ok {
        if let Some(prev) = prev {
            let _ = fs::write(LOCAL_AI_NGINX_CONF, prev);
        } else {
            let _ = fs::remove_file(LOCAL_AI_NGINX_CONF);
        }
        return Err("nginx validation failed".into());
    }
    let _ = Command::new(SYSTEMCTL_BIN)
        .args(["reload", "nginx"])
        .status();
    if let Some(parent) = Path::new(LOCAL_AI_FIREWALL_RECEIPT).parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(
        LOCAL_AI_FIREWALL_RECEIPT,
        format!(
            "lan_port={}\nlan_cidr={}\npublic_exposure=false\n",
            cfg.lan_port, cfg.lan_cidr
        ),
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

