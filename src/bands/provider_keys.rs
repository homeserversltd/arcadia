async fn provider_keys_status() -> Json<ProviderKeysStatusResponse> {
    let configured = read_provider_key_presence();
    let providers = PROVIDER_KEY_NAMES
        .iter()
        .map(|(id, key)| ProviderKeyStatus {
            id: *id,
            env_key: *key,
            configured: configured.get(*key).copied().unwrap_or(false),
        })
        .collect::<Vec<_>>();
    Json(ProviderKeysStatusResponse {
        ok: true,
        action: "provider-keys-status",
        path: PROVIDER_KEYS_PATH,
        providers,
        message: "Provider key status loaded without exposing secret values.".to_string(),
    })
}

fn read_provider_key_presence() -> HashMap<String, bool> {
    let mut result = HashMap::new();
    let Ok(text) = fs::read_to_string(PROVIDER_KEYS_PATH) else {
        return result;
    };
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, raw_value)) = trimmed.split_once('=') else {
            continue;
        };
        let value = raw_value.trim().trim_matches('"').trim_matches('\'');
        if !value.is_empty() {
            result.insert(key.trim().to_string(), true);
        }
    }
    result
}

async fn save_provider_keys(
    Json(body): Json<ProviderKeysRequest>,
) -> (StatusCode, Json<ProviderKeysResponse>) {
    let mut lines: Vec<String> = Vec::new();
    let mut written: Vec<&'static str> = Vec::new();
    push_env_value(
        &mut lines,
        &mut written,
        "STEAMGRIDDB_API_KEY",
        body.steamgriddb_api_key,
    );
    push_env_value(
        &mut lines,
        &mut written,
        "THEGAMESDB_API_KEY",
        body.thegamesdb_api_key,
    );
    push_env_value(
        &mut lines,
        &mut written,
        "SCREENSCRAPER_API_KEY",
        body.screenscraper_api_key,
    );

    if lines.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ProviderKeysResponse {
                ok: false,
                action: "save-provider-keys",
                path: PROVIDER_KEYS_PATH,
                written_keys: Vec::new(),
                message: "Enter at least one provider key.".to_string(),
            }),
        );
    }

    if let Some(parent) = Path::new(PROVIDER_KEYS_PATH).parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            return provider_keys_error(format!(
                "Provider key directory could not be created: {err}"
            ));
        }
    }

    let write_result = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(PROVIDER_KEYS_PATH)
        .and_then(|mut file| file.write_all(lines.join("\n").as_bytes()))
        .and_then(|_| fs::set_permissions(PROVIDER_KEYS_PATH, provider_file_permissions()));

    match write_result {
        Ok(()) => (
            StatusCode::OK,
            Json(ProviderKeysResponse {
                ok: true,
                action: "save-provider-keys",
                path: PROVIDER_KEYS_PATH,
                written_keys: written,
                message: "Provider keys saved for game sync.".to_string(),
            }),
        ),
        Err(err) => provider_keys_error(format!("Provider keys could not be saved: {err}")),
    }
}

fn push_env_value(
    lines: &mut Vec<String>,
    written: &mut Vec<&'static str>,
    key: &'static str,
    value: Option<String>,
) {
    let Some(value) = value else { return };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return;
    }
    lines.push(format!("{}={}", key, shell_env_quote(trimmed)));
    written.push(key);
}

fn shell_env_quote(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{}\"", escaped)
}

fn provider_keys_error(message: String) -> (StatusCode, Json<ProviderKeysResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ProviderKeysResponse {
            ok: false,
            action: "save-provider-keys",
            path: PROVIDER_KEYS_PATH,
            written_keys: Vec::new(),
            message,
        }),
    )
}

#[cfg(unix)]
fn provider_file_permissions() -> fs::Permissions {
    fs::Permissions::from_mode(0o600)
}

#[cfg(not(unix))]
fn provider_file_permissions() -> fs::Permissions {
    fs::metadata(PROVIDER_KEYS_PATH)
        .map(|m| m.permissions())
        .unwrap_or_else(|_| fs::Permissions::readonly())
}
