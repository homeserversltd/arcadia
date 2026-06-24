#[derive(Default, Clone)]
struct ActiveNetInfo {
    kind: String,
    interface_name: Option<String>,
    ip: Option<String>,
    prefix_length: Option<u8>,
    ssid: Option<String>,
    signal_percent: Option<u8>,
    security: Option<String>,
}

fn connection_label(kind: &str) -> &'static str {
    match kind {
        "ethernet" => "Ethernet",
        "wifi" => "Wi-Fi",
        "limited" => "Limited",
        "offline" => "Offline",
        _ => "Unknown",
    }
}

fn nmcli_device_rows() -> Vec<Vec<String>> {
    command_stdout(
        NETWORK_MANAGER_BIN,
        &[
            "-t",
            "-f",
            "DEVICE,TYPE,STATE,CONNECTION",
            "device",
            "status",
        ],
    )
    .map(|text| text.lines().map(split_nmcli_line).collect())
    .unwrap_or_default()
}

fn nmcli_connection_rows() -> Vec<Vec<String>> {
    command_stdout(
        NETWORK_MANAGER_BIN,
        &["-t", "-f", "NAME,TYPE,TIMESTAMP", "connection", "show"],
    )
    .map(|text| text.lines().map(split_nmcli_line).collect())
    .unwrap_or_default()
}

fn split_nmcli_line(line: &str) -> Vec<String> {
    line.split(':').map(|v| v.replace("\\:", ":")).collect()
}

fn nmcli_active_connection_rows() -> Vec<Vec<String>> {
    command_stdout(
        NETWORK_MANAGER_BIN,
        &[
            "-t",
            "-f",
            "NAME,TYPE,DEVICE,TIMESTAMP",
            "connection",
            "show",
            "--active",
        ],
    )
    .map(|text| {
        text.lines()
            .filter(|line| !line.trim().is_empty())
            .map(split_nmcli_line)
            .collect()
    })
    .unwrap_or_default()
}

fn connection_session_detail(active_type: &str, ssid: Option<&str>) -> String {
    let duration = active_connection_duration(active_type);
    match active_type {
        "ethernet" => format!(
            "Ethernet · {}",
            duration.as_deref().unwrap_or("just now")
        ),
        "wifi" => format!(
            "Wi-Fi · {} · {}",
            ssid.unwrap_or("Wi-Fi"),
            duration.as_deref().unwrap_or("just now")
        ),
        "limited" => format!(
            "Limited · {}",
            duration.as_deref().unwrap_or("just now")
        ),
        "offline" => "Offline".to_string(),
        _ => "Unknown".to_string(),
    }
}

fn active_connection_duration(active_type: &str) -> Option<String> {
    let want_type = match active_type {
        "ethernet" => "802-3-ethernet",
        "wifi" => "802-11-wireless",
        _ => return None,
    };
    let timestamp = nmcli_active_connection_rows().into_iter().find_map(|parts| {
        let conn_type = parts.get(1).map(String::as_str).unwrap_or("");
        let device = parts.get(2).map(String::as_str).unwrap_or("");
        if device == "lo" || conn_type != want_type {
            return None;
        }
        parts.get(3).and_then(|value| value.parse::<u64>().ok())
    })?;
    Some(format_connection_duration(timestamp))
}

fn format_connection_duration(connected_since_unix: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(connected_since_unix);
    let seconds = now.saturating_sub(connected_since_unix);
    if seconds < 60 {
        return format!("{seconds}s");
    }
    if seconds < 3600 {
        return format!("{}m", seconds / 60);
    }
    if seconds < 86_400 {
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        return if minutes == 0 {
            format!("{hours}h")
        } else {
            format!("{hours}h {minutes}m")
        };
    }
    let days = seconds / 86_400;
    let hours = (seconds % 86_400) / 3600;
    if hours == 0 {
        format!("{days}d")
    } else {
        format!("{days}d {hours}h")
    }
}

fn active_connection_from_nmcli(devices: &[Vec<String>]) -> ActiveNetInfo {
    for parts in devices {
        let dev = parts.get(0).cloned().unwrap_or_default();
        let kind = parts.get(1).cloned().unwrap_or_default();
        let state = parts.get(2).cloned().unwrap_or_default();
        if state != "connected" || dev == "lo" {
            continue;
        }
        let ip = interface_ipv4(&dev);
        let mut info = ActiveNetInfo {
            kind: if kind == "wifi" {
                "wifi".into()
            } else if kind == "ethernet" {
                "ethernet".into()
            } else {
                "unknown".into()
            },
            interface_name: Some(dev.clone()),
            prefix_length: interface_prefix(&dev),
            ip,
            ..Default::default()
        };
        if kind == "wifi" {
            info.ssid = parts
                .get(3)
                .cloned()
                .filter(|v| !v.is_empty() && v != "--")
                .or_else(|| wifi_ssid(&dev));
            info.signal_percent = wifi_signal_for_ssid(info.ssid.as_deref());
            info.security = wifi_security_for_ssid(info.ssid.as_deref());
        }
        return info;
    }
    ActiveNetInfo::default()
}

fn interface_ipv4(dev: &str) -> Option<String> {
    command_stdout(
        "ip",
        &["-4", "-o", "addr", "show", "dev", dev, "scope", "global"],
    )
    .and_then(|text| parse_global_ipv4_address(&text))
}

fn interface_prefix(dev: &str) -> Option<u8> {
    command_stdout(
        "ip",
        &["-4", "-o", "addr", "show", "dev", dev, "scope", "global"],
    )
    .and_then(|text| {
        text.lines().find_map(|line| {
            line.split_whitespace()
                .collect::<Vec<_>>()
                .windows(2)
                .find_map(|w| {
                    if w[0] == "inet" {
                        w[1].split('/').nth(1)?.parse().ok()
                    } else {
                        None
                    }
                })
        })
    })
}

fn default_gateway() -> Option<String> {
    command_stdout("ip", &["route", "show", "default"]).and_then(|text| {
        let parts: Vec<&str> = text.split_whitespace().collect();
        parts.windows(2).find_map(|w| {
            if w[0] == "via" {
                Some(w[1].to_string())
            } else {
                None
            }
        })
    })
}

fn dns_servers() -> Vec<String> {
    fs::read_to_string("/etc/resolv.conf")
        .unwrap_or_default()
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            if parts.next()? == "nameserver" {
                parts.next().map(str::to_string)
            } else {
                None
            }
        })
        .filter(|v| valid_ipv4(v))
        .collect()
}

fn ethernet_state(
    devices: &[Vec<String>],
    active: &ActiveNetInfo,
    dns: &[String],
    gateway: Option<String>,
) -> EthernetState {
    let iface = devices
        .iter()
        .find(|p| p.get(1).map(|v| v == "ethernet").unwrap_or(false))
        .and_then(|p| p.first())
        .cloned()
        .or_else(ethernet_interface_name);
    let connected = iface
        .as_ref()
        .map(|name| active.interface_name.as_deref() == Some(name.as_str()) || carrier_up(name))
        .unwrap_or(false);
    let ip = iface.as_ref().and_then(|name| interface_ipv4(name));
    EthernetState {
        available: iface.is_some(),
        connected,
        interface_name: iface.clone(),
        mac_address: iface
            .as_ref()
            .and_then(|name| fs::read_to_string(format!("/sys/class/net/{}/address", name)).ok())
            .map(|v| v.trim().to_string()),
        speed_mbps: iface.as_ref().and_then(|name| read_speed_mbps(name)),
        ip,
        dhcp: true,
        gateway,
        dns_servers: dns.to_vec(),
    }
}

fn wifi_state(
    _devices: &[Vec<String>],
    active: &ActiveNetInfo,
    saved: &[SavedWifiNetwork],
    mut scan_results: Vec<WifiScanResult>,
) -> WifiState {
    let adapter = wifi_adapter_name();
    for row in &mut scan_results {
        if active.ssid.as_deref() == Some(row.ssid.as_str()) {
            row.connected = true;
        }
        if saved.iter().any(|s| s.ssid == row.ssid) {
            row.saved = true;
        }
    }
    WifiState {
        adapter_available: adapter.is_some(),
        enabled: wifi_enabled(),
        scanning: false,
        connected_ssid: active.ssid.clone(),
        signal_percent: active.signal_percent,
        security: active.security.clone(),
        saved_networks: saved.to_vec(),
        scan_results,
    }
}

fn saved_wifi_networks(conns: &[Vec<String>]) -> Vec<SavedWifiNetwork> {
    conns
        .iter()
        .filter(|p| {
            p.get(1)
                .map(|v| v.contains("wireless") || v == "wifi")
                .unwrap_or(false)
        })
        .filter_map(|p| p.first().cloned())
        .map(|ssid| SavedWifiNetwork {
            ssid,
            security: None,
            last_connected_at: None,
        })
        .collect()
}

fn wifi_scan_results(saved: &[SavedWifiNetwork]) -> Vec<WifiScanResult> {
    let Some(text) = command_stdout(
        NETWORK_MANAGER_BIN,
        &[
            "-t",
            "-f",
            "SSID,BSSID,SIGNAL,SECURITY,IN-USE",
            "device",
            "wifi",
            "list",
        ],
    ) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| {
            let p = split_nmcli_line(line);
            let ssid = p.first()?.trim().to_string();
            if ssid.is_empty() {
                return None;
            }
            let signal = p
                .get(2)
                .and_then(|v| v.parse::<u8>().ok())
                .unwrap_or(0)
                .min(100);
            let security = normalize_wifi_security(p.get(3).map(String::as_str).unwrap_or(""));
            Some(WifiScanResult {
                ssid: ssid.clone(),
                bssid: p.get(1).cloned().filter(|v| !v.is_empty()),
                signal_percent: signal,
                security,
                saved: saved.iter().any(|s| s.ssid == ssid),
                connected: p.get(4).map(|v| v == "*").unwrap_or(false),
            })
        })
        .collect()
}

fn normalize_wifi_security(raw: &str) -> String {
    let value = raw.to_ascii_lowercase();
    if value.trim().is_empty() || value == "--" {
        "open".into()
    } else if value.contains("wpa3") {
        "wpa3".into()
    } else if value.contains("wpa2") && value.contains("wpa1") {
        "wpa-wpa2".into()
    } else if value.contains("wpa2") || value.contains("wpa") {
        "wpa2".into()
    } else {
        "unknown".into()
    }
}

fn wifi_enabled() -> bool {
    command_stdout(NETWORK_MANAGER_BIN, &["radio", "wifi"])
        .map(|v| v.trim() == "enabled")
        .unwrap_or_else(|| wifi_adapter_name().is_some())
}

fn wifi_ssid(dev: &str) -> Option<String> {
    command_stdout("iw", &["dev", dev, "link"]).and_then(|text| {
        text.lines()
            .find_map(|line| line.trim().strip_prefix("SSID: ").map(str::to_string))
    })
}

fn wifi_signal_for_ssid(ssid: Option<&str>) -> Option<u8> {
    let ssid = ssid?;
    wifi_scan_results(&[])
        .into_iter()
        .find(|row| row.ssid == ssid)
        .map(|row| row.signal_percent)
}

fn wifi_security_for_ssid(ssid: Option<&str>) -> Option<String> {
    let ssid = ssid?;
    wifi_scan_results(&[])
        .into_iter()
        .find(|row| row.ssid == ssid)
        .map(|row| row.security)
}

fn ethernet_interface_name() -> Option<String> {
    fs::read_dir("/sys/class/net")
        .ok()?
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .find(|n| n != "lo" && !n.starts_with("wl") && !n.starts_with("wifi"))
}

fn carrier_up(name: &str) -> bool {
    fs::read_to_string(format!("/sys/class/net/{}/carrier", name))
        .map(|v| v.trim() == "1")
        .unwrap_or(false)
}
fn read_speed_mbps(name: &str) -> Option<u64> {
    fs::read_to_string(format!("/sys/class/net/{}/speed", name))
        .ok()?
        .trim()
        .parse()
        .ok()
        .filter(|v| *v > 0)
}

fn clock_status() -> (Option<String>, Option<bool>) {
    let timezone = command_stdout("timedatectl", &["show", "-p", "Timezone", "--value"]);
    let ntp_synchronized = command_stdout("timedatectl", &["show", "-p", "NTPSynchronized", "--value"])
        .map(|value| value.trim() == "yes");
    (timezone, ntp_synchronized)
}

const SPEED_TEST_BYTES: u64 = 25_000_000;

fn run_download_speed_test() -> SpeedTestResponse {
    if !internet_reachable() {
        return SpeedTestResponse {
            ok: false,
            action: "network-speed-test",
            download_mbps: None,
            duration_ms: None,
            bytes: None,
            message: "Internet unavailable. Connect to run a speed test.".to_string(),
        };
    }
    let url = format!(
        "https://speed.cloudflare.com/__down?bytes={SPEED_TEST_BYTES}"
    );
    let started = std::time::Instant::now();
    let output = std::process::Command::new("curl")
        .args([
            "-fsS",
            "-o",
            "/dev/null",
            "-w",
            "%{size_download}",
            "--max-time",
            "30",
            &url,
        ])
        .output();
    let duration_ms = started.elapsed().as_millis() as u64;
    let Ok(output) = output else {
        return SpeedTestResponse {
            ok: false,
            action: "network-speed-test",
            download_mbps: None,
            duration_ms: Some(duration_ms),
            bytes: None,
            message: "Speed test could not start.".to_string(),
        };
    };
    if !output.status.success() {
        return SpeedTestResponse {
            ok: false,
            action: "network-speed-test",
            download_mbps: None,
            duration_ms: Some(duration_ms),
            bytes: None,
            message: "Speed test failed. Check Internet and try again.".to_string(),
        };
    }
    let bytes = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<f64>()
        .ok()
        .map(|v| v as u64)
        .filter(|v| *v > 0);
    let Some(bytes) = bytes else {
        return SpeedTestResponse {
            ok: false,
            action: "network-speed-test",
            download_mbps: None,
            duration_ms: Some(duration_ms),
            bytes: None,
            message: "Speed test returned no data.".to_string(),
        };
    };
    let seconds = duration_ms.max(1) as f64 / 1000.0;
    let download_mbps = (bytes as f64 * 8.0 / 1_000_000.0) / seconds;
    SpeedTestResponse {
        ok: true,
        action: "network-speed-test",
        download_mbps: Some((download_mbps * 10.0).round() / 10.0),
        duration_ms: Some(duration_ms),
        bytes: Some(bytes),
        message: format!("{:.1} Mbps download", download_mbps),
    }
}

