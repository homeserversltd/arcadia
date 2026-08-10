fn console_status(state: &AppState) -> ConsoleStatus {
    let storage = storage_status();
    let network = network_status();
    let (surfaces, samba) = surface_and_samba_status(&state.canonical_url, &network);
    let library = library_status(&storage);
    let hostname = hostname();
    ConsoleStatus {
        schema: "arcadia.home_state.v2",
        product: state.product.clone(),
        canonical_url: state.canonical_url.clone(),
        identity: IdentityStatus {
            product_name: state.product.clone(),
            hostname: hostname.clone(),
            local_domain: Some("home.arpa".to_string()),
            netbios_name: Some("HOMECONSOLE".to_string()),
            web_origin: state.canonical_url.trim_end_matches('/').to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        arcadia: ArcadiaStatus {
            service: service_state("arcadia.service"),
            version: env!("CARGO_PKG_VERSION"),
            mode: "unity-appliance-shell",
            ui: "top-header-left-launcher-focused-viewports",
        },
        runtime: runtime_status(state.started_unix),
        gui_pin: gui_pin_status(),
        vault: vault_status(),
        surfaces,
        samba,
        network: network.clone(),
        local_ai: local_ai_status(),
        controllers: controller_status(),
        updates: updates_status(),
        system: system_admin_status(&network, &hostname),
        benchmark: benchmark_status(),
        library,
        storage,
        ui_contract: UiContract {
            schema: "arcadia.ui.contract.v6",
            button_variants: ["primary", "secondary", "danger"],
            composition: "top header, sidebar launcher, state-first operational viewport",
            modal: "confirmation/readback only; GUI PIN changes post to local root-owned helpers",
        },
    }
}

fn service_state(unit: &'static str) -> &'static str {
    let Ok(output) = Command::new(SYSTEMCTL_BIN)
        .args(["is-active", unit])
        .output()
    else {
        return "unknown";
    };
    let state = String::from_utf8_lossy(&output.stdout);
    match state.trim() {
        "active" => "running",
        "inactive" | "failed" => "stopped",
        "activating" => "starting",
        _ => "unknown",
    }
}

fn command_stdout(command: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(command).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn parse_global_ipv4_address(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        while let Some(part) = parts.next() {
            if part == "inet" {
                return parts
                    .next()
                    .and_then(|cidr| cidr.split('/').next())
                    .filter(|addr| !addr.is_empty())
                    .map(str::to_string);
            }
        }
        None
    })
}

fn network_status() -> NetworkStatus {
    let state = network_state_from_parts("HomeConsole", None);
    let resolv = read_resolv_conf();
    let (timezone, ntp_synchronized) = clock_status();
    let online = state.active_connection.connection_type != "offline";
    NetworkStatus {
        online,
        active_type: state.active_connection.connection_type.clone(),
        connection_type: connection_label(&state.active_connection.connection_type).to_string(),
        ssid: state.wifi.connected_ssid.clone(),
        ip_address: state
            .active_connection
            .ip
            .clone()
            .unwrap_or_else(|| "—".to_string()),
        gateway: state.active_connection.gateway.clone(),
        dns_status: if state.active_connection.dns_servers.is_empty() {
            "Unknown".to_string()
        } else {
            "DNS working".to_string()
        },
        signal: state.wifi.signal_percent.map(|v| format!("{}%", v)),
        signal_percent: state.wifi.signal_percent,
        ethernet_speed_mbps: state.ethernet.speed_mbps,
        ethernet_available: state.ethernet.available,
        ethernet_connected: state.ethernet.connected,
        ethernet_mac_address: state.ethernet.mac_address.clone(),
        ethernet_dhcp: state.ethernet.dhcp,
        wifi_adapter_available: state.wifi.adapter_available,
        console_reachable: state.active_connection.lan_reachable,
        game_folders_reachable: state.services.samba.state == "available",
        samba_reachable: state.services.samba.state == "available",
        lan_ai_reachable: state.services.lan_inference.state == "available",
        internet_reachable: state.active_connection.internet_reachable,
        timezone,
        ntp_synchronized,
        connection_session_detail: connection_session_detail(
            &state.active_connection.connection_type,
            state.wifi.connected_ssid.as_deref(),
        ),
        resolv_nameservers: resolv_nameservers_label(&resolv),
        resolv_search: resolv_search_label(&resolv),
    }
}

fn network_state(state: &AppState) -> NetworkState {
    network_state_from_parts(
        &state.product,
        Some(state.canonical_url.trim_end_matches('/')),
    )
}

fn network_state_from_parts(product: &str, canonical_url: Option<&str>) -> NetworkState {
    let hostname = hostname();
    let web_origin = canonical_url
        .map(str::to_string)
        .unwrap_or_else(|| format!("http://{}.home.arpa", hostname));
    let local_domain = format!("{}.home.arpa", hostname);
    let netbios = netbios_name(&hostname);
    let devices = nmcli_device_rows();
    let conns = nmcli_connection_rows();
    let saved = saved_wifi_networks(&conns);
    let scan = wifi_scan_results(&saved);
    let active = active_connection_from_nmcli(&devices);
    let dns_servers = dns_servers();
    let gateway = default_gateway();
    let ethernet = ethernet_state(&devices, &active, &dns_servers, gateway.clone());
    let wifi = wifi_state(&devices, &active, &saved, scan);
    let lan_reachable = active.ip.is_some() || gateway.is_some();
    let internet = lan_reachable.then(internet_reachable);
    let active_connection = ActiveConnection {
        connection_type: if active.kind == "wifi" {
            "wifi"
        } else if active.kind == "ethernet" {
            "ethernet"
        } else if lan_reachable {
            "limited"
        } else {
            "offline"
        }
        .to_string(),
        interface_name: active.interface_name.clone(),
        ip: active.ip.clone(),
        prefix_length: active.prefix_length,
        gateway,
        dns_servers: dns_servers.clone(),
        internet_reachable: internet,
        lan_reachable,
    };
    let services = network_services(&web_origin, active.ip.as_deref(), &hostname, &netbios);
    NetworkState {
        appliance: NetworkAppliance {
            product_name: product.to_string(),
            hostname,
            local_domain: Some(local_domain),
            netbios_name: Some(netbios),
            web_origin,
        },
        active_connection,
        ethernet,
        wifi,
        services,
    }
}
