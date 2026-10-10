fn system_admin_status(network: &NetworkStatus, hostname: &str) -> SystemAdminStatus {
    SystemAdminStatus {
        ssh: ssh_access_status(network, hostname),
        trust: trust_status(),
        services: system_service_statuses(),
    }
}

fn ssh_access_status(network: &NetworkStatus, hostname: &str) -> SshAccessStatus {
    let username = ssh_username();
    let host = if hostname.is_empty() {
        "console"
    } else {
        hostname
    };
    let domain = format!("{}.home.arpa", host);
    SshAccessStatus {
        service_state: if service_state("sshd.service") == "running" || tcp_port_listening(22) {
            "enabled"
        } else {
            "disabled"
        }
        .to_string(),
        password_auth: ssh_password_auth_state(),
        hostname: domain.clone(),
        username: username.clone(),
        lan_ip: network.ip_address.clone(),
        command: format!("ssh {}@{}", username, domain),
        authorized_keys_path: Path::new(&user_home(&username))
            .join(".ssh/authorized_keys")
            .display()
            .to_string(),
    }
}

fn ssh_username() -> String {
    env::var("HOMECONSOLE_SSH_USER").unwrap_or_else(|_| "owner".to_string())
}

fn user_home(user: &str) -> String {
    if user == "root" {
        "/root".to_string()
    } else {
        format!("/home/{}", user)
    }
}

fn ssh_password_auth_state() -> String {
    let text = fs::read_to_string(SSHD_CONFIG_MANAGED_PATH)
        .or_else(|_| fs::read_to_string("/etc/ssh/sshd_config"))
        .unwrap_or_default();
    for line in text.lines().rev() {
        let clean = line.trim();
        if clean.starts_with('#') {
            continue;
        }
        let lower = clean.to_ascii_lowercase();
        if lower.starts_with("passwordauthentication") {
            return if lower.split_whitespace().nth(1) == Some("yes") {
                "enabled"
            } else {
                "disabled"
            }
            .to_string();
        }
    }
    "unknown".to_string()
}

const HOUSEHOLD_TRUST_CACHE_TTL: Duration = Duration::from_secs(30);
const HOUSEHOLD_CA_ANCHOR_PATHS: [&str; 2] = [
    "/etc/ca-certificates/trust-source/anchors/homeserver-house-ca.crt",
    HOMECONSOLE_CA_ANCHOR_PATH,
];

#[derive(Clone, Default)]
struct CachedCaduceusTrustStatus {
    observed_at: Option<std::time::Instant>,
    available: bool,
    bundle_installed: bool,
    ring_fingerprint: Option<String>,
}

#[derive(Clone, Default)]
struct CachedHouseholdInteractables {
    observed_at: Option<std::time::Instant>,
    renew: Option<TrustRenewEvidence>,
}

#[derive(Default)]
struct HouseholdTrustCache {
    status: CachedCaduceusTrustStatus,
    interactables: CachedHouseholdInteractables,
}

static HOUSEHOLD_TRUST_CACHE: OnceLock<Mutex<HouseholdTrustCache>> = OnceLock::new();

fn household_trust_cache() -> &'static Mutex<HouseholdTrustCache> {
    HOUSEHOLD_TRUST_CACHE.get_or_init(|| Mutex::new(HouseholdTrustCache::default()))
}

fn household_trust_status_observation(value: &serde_json::Value) -> CachedCaduceusTrustStatus {
    let bundle_installed = value
        .get("bundle_installed")
        .and_then(serde_json::Value::as_bool);
    let available = value.get("ok").and_then(serde_json::Value::as_bool) != Some(false)
        && bundle_installed.is_some();
    CachedCaduceusTrustStatus {
        observed_at: Some(std::time::Instant::now()),
        available,
        bundle_installed: bundle_installed.unwrap_or(false),
        ring_fingerprint: available
            .then(|| {
                [
                    "ring_fingerprint",
                    "ca_fingerprint",
                    "fingerprint",
                    "bundle_fingerprint",
                ]
                .iter()
                .find_map(|field| {
                    value
                        .get(*field)
                        .and_then(serde_json::Value::as_str)
                        .filter(|fingerprint| !fingerprint.is_empty())
                })
            })
            .flatten()
            .map(str::to_string),
    }
}

fn record_household_trust_status(value: &serde_json::Value) {
    household_trust_cache()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .status = household_trust_status_observation(value);
}

fn record_household_trust_unavailable() {
    household_trust_cache()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .status = CachedCaduceusTrustStatus {
        observed_at: Some(std::time::Instant::now()),
        ..CachedCaduceusTrustStatus::default()
    };
}

fn household_renew_evidence(value: &serde_json::Value) -> Option<TrustRenewEvidence> {
    let item = value
        .get("interactables")?
        .as_array()?
        .iter()
        .find(|item| {
            item.get("id").and_then(serde_json::Value::as_str) == Some("household-trust-renew")
                && item.get("kind").and_then(serde_json::Value::as_str)
                    == Some("household-trust-renew")
        })?;
    let evidence = item.get("evidence")?;
    Some(TrustRenewEvidence {
        recorded_fingerprint: evidence
            .get("recorded_fingerprint")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        served_fingerprint: evidence
            .get("served_fingerprint")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        gateway_seat: evidence
            .get("gateway_seat")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    })
}

fn record_household_interactables(value: &serde_json::Value) {
    household_trust_cache()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .interactables = CachedHouseholdInteractables {
        observed_at: Some(std::time::Instant::now()),
        renew: household_renew_evidence(value),
    };
}

fn record_household_interactables_unavailable() {
    household_trust_cache()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .interactables = CachedHouseholdInteractables {
        observed_at: Some(std::time::Instant::now()),
        renew: None,
    };
}

fn household_cache_entry_stale(observed_at: Option<std::time::Instant>, force: bool) -> bool {
    force
        || observed_at.map_or(true, |observed_at| {
            observed_at.elapsed() >= HOUSEHOLD_TRUST_CACHE_TTL
        })
}

fn invalidate_household_trust_cache() {
    let mut cache = household_trust_cache()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    cache.status.observed_at = None;
    cache.interactables.observed_at = None;
}

fn refresh_household_trust_cache(
    force: bool,
) -> (CachedCaduceusTrustStatus, CachedHouseholdInteractables) {
    let mut cache = household_trust_cache()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let client = CaduceusAccessClient::default();
    if household_cache_entry_stale(cache.status.observed_at, force) {
        match client.get_json("/api/v1/cert/status") {
            Ok(value) => cache.status = household_trust_status_observation(&value),
            Err(_) => {
                cache.status = CachedCaduceusTrustStatus {
                    observed_at: Some(std::time::Instant::now()),
                    ..CachedCaduceusTrustStatus::default()
                };
            }
        }
    }
    if household_cache_entry_stale(cache.interactables.observed_at, force) {
        cache.interactables = match client.get_json("/api/v1/interactables") {
            Ok(value) => CachedHouseholdInteractables {
                observed_at: Some(std::time::Instant::now()),
                renew: household_renew_evidence(&value),
            },
            Err(_) => CachedHouseholdInteractables {
                observed_at: Some(std::time::Instant::now()),
                renew: None,
            },
        };
    }
    (cache.status.clone(), cache.interactables.clone())
}

fn existing_household_ca_anchor() -> Option<&'static str> {
    HOUSEHOLD_CA_ANCHOR_PATHS
        .into_iter()
        .find(|path| Path::new(path).is_file())
}

fn trust_status() -> TrustStatus {
    let mode = fs::read_to_string(TRUST_MODE_PATH)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|v| v.get("mode").and_then(|m| m.as_str()).map(str::to_string))
        .filter(|m| m == "http" || m == "https")
        .unwrap_or_else(|| "http".to_string());
    let (caduceus, interactables) = refresh_household_trust_cache(false);
    let ca_path = existing_household_ca_anchor();
    TrustStatus {
        mode,
        ca_installed: if caduceus.available {
            caduceus.bundle_installed
        } else {
            ca_path.is_some()
        },
        ca_subject: ca_metadata("subject", ca_path),
        ca_issuer: ca_metadata("issuer", ca_path),
        ca_not_after: ca_metadata("enddate", ca_path),
        ca_path,
        https_probe_url: HOME_ROOT_HTTPS_PROBE,
        caduceus_available: caduceus.available,
        ring_fingerprint: caduceus.ring_fingerprint,
        renew: interactables.renew,
    }
}

fn ca_metadata(field: &str, path: Option<&str>) -> Option<String> {
    let path = path?;
    let arg = match field {
        "subject" => "-subject",
        "issuer" => "-issuer",
        "enddate" => "-enddate",
        _ => return None,
    };
    command_stdout("openssl", &["x509", "-in", path, "-noout", arg]).map(|s| {
        s.replace("subject=", "")
            .replace("issuer=", "")
            .replace("notAfter=", "")
    })
}

fn ca_metadata_text() -> String {
    let t = trust_status();
    [
        t.ca_subject.map(|v| format!("subject={v}")),
        t.ca_issuer.map(|v| format!("issuer={v}")),
        t.ca_not_after.map(|v| format!("not_after={v}")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join("\n")
}

fn https_probe_ok() -> bool {
    Command::new("curl")
        .args(["-fsS", "--max-time", "5", HOME_ROOT_HTTPS_PROBE])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn system_service_statuses() -> Vec<SystemServiceStatus> {
    vec![
        system_service_status("Game Session", "gamescope.service", "Game display", None, None),
        system_service_status("Game Folders", "smb.service", "File sharing", None, None),
        system_service_status(
            "Game Sync",
            "homeconsole-sync.service",
            "Runs on demand",
            None,
            None,
        ),
        system_service_status(
            "Local AI",
            "llama-server.service",
            "Model and LAN inference runtime",
            None,
            None,
        ),
        system_service_status(
            "HomeConsole Interface",
            "arcadia.service",
            "Web front panel",
            None,
            None,
        ),
    ]
}

fn system_service_status(
    name: &str,
    unit: &'static str,
    detail: &str,
    action: Option<&str>,
    endpoint: Option<&str>,
) -> SystemServiceStatus {
    SystemServiceStatus {
        name: name.to_string(),
        state: service_state(unit).to_string(),
        detail: detail.to_string(),
        action: action.map(str::to_string),
        endpoint: endpoint.map(str::to_string),
    }
}

fn normalize_public_key(raw: &str) -> Option<String> {
    let line = raw.lines().find(|l| !l.trim().is_empty())?.trim();
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 2 || parts.len() > 3 {
        return None;
    }
    if !matches!(
        parts[0],
        "ssh-ed25519"
            | "ssh-rsa"
            | "ecdsa-sha2-nistp256"
            | "ecdsa-sha2-nistp384"
            | "ecdsa-sha2-nistp521"
    ) {
        return None;
    }
    if parts[1].len() < 32
        || parts[1]
            .chars()
            .any(|c| !(c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '='))
    {
        return None;
    }
    Some(parts.join(" "))
}

fn normalize_ca_bundle(raw: &str) -> Option<String> {
    let text = raw.trim().replace("\r\n", "\n");
    if text.contains("-----BEGIN CERTIFICATE-----") && text.contains("-----END CERTIFICATE-----") {
        Some(format!("{}\n", text))
    } else {
        None
    }
}

fn set_mode(path: &Path, mode: u32) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        fs::set_permissions(path, fs::Permissions::from_mode(mode))
    }
    #[cfg(not(unix))]
    {
        let _ = mode;
        let _ = path;
        Ok(())
    }
}

fn chown_path(path: &Path, user: &str) -> std::io::Result<()> {
    let spec = format!("{}:{}", user, user);
    let _ = Command::new("chown").arg(&spec).arg(path).output();
    Ok(())
}

fn network_services(
    web_origin: &str,
    ip: Option<&str>,
    hostname: &str,
    netbios: &str,
) -> NetworkServices {
    let samba_ok = samba_available() && Path::new(GAMES_ROOT).exists();
    let share = "games";
    let mut urls = vec![web_origin.to_string()];
    if let Some(ip) = ip {
        urls.push(format!("http://{}", ip));
    }
    let share_row = SambaShareStatus {
        name: share.to_string(),
        purpose: "games".to_string(),
        windows_unc: samba_ok.then(|| format!(r"\\{}\{}", netbios, share)),
        windows_unc_by_ip: samba_ok
            .then(|| ip.map(|addr| format!(r"\\{}\{}", addr, share)))
            .flatten(),
        smb_url: samba_ok.then(|| format!("smb://{}/{}", hostname, share)),
        smb_url_by_ip: samba_ok
            .then(|| ip.map(|addr| format!("smb://{}/{}", addr, share)))
            .flatten(),
    };
    let cfg = load_ai_config();
    let lan_ai = cfg.lan_enabled && tcp_port_listening(cfg.lan_port);
    NetworkServices {
        web_console: WebConsoleService {
            state: "available".to_string(),
            urls,
        },
        samba: SambaServiceState {
            state: if samba_ok {
                "available"
            } else if SAMBA_SERVICE_NAMES
                .iter()
                .any(|unit| service_state(unit) == "unknown")
            {
                "unknown"
            } else {
                "disabled"
            }
            .to_string(),
            shares: samba_ok.then(|| vec![share_row]).unwrap_or_default(),
        },
        lan_inference: LanInferenceService {
            state: if lan_ai { "available" } else { "disabled" }.to_string(),
            port: cfg.lan_port,
            urls: lan_ai
                .then(|| vec![format!("{}:{}", web_origin, cfg.lan_port)])
                .unwrap_or_default(),
        },
        ssh: SshService {
            state: if service_state("sshd.service") == "running" || tcp_port_listening(22) {
                "available"
            } else {
                "disabled"
            }
            .to_string(),
            port: 22,
        },
    }
}

fn netbios_name(hostname: &str) -> String {
    let cleaned: String = hostname
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect::<String>()
        .to_ascii_uppercase();
    if cleaned.is_empty() {
        "HOMECONSOLE".to_string()
    } else {
        cleaned.chars().take(15).collect()
    }
}

fn internet_reachable() -> bool {
    let Ok(addr) = "1.1.1.1:53".parse() else {
        return false;
    };
    TcpStream::connect_timeout(&addr, Duration::from_millis(180)).is_ok()
}

fn tcp_port_listening(port: u16) -> bool {
    let needle = format!(":{:04X}", port);
    fs::read_to_string("/proc/net/tcp")
        .map(|text| {
            text.lines()
                .skip(1)
                .any(|line| line.contains(&needle) && line.split_whitespace().nth(3) == Some("0A"))
        })
        .unwrap_or(false)
}
