async fn wifi_status(State(state): State<Arc<AppState>>) -> Json<NetworkState> {
    Json(state.living_snapshot().network.clone())
}

async fn wifi_scan(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<NetworkActionResponse>) {
    if !state
        .living_snapshot()
        .network
        .clone()
        .wifi
        .adapter_available
        || !helper_exists(NETWORK_MANAGER_BIN)
    {
        return network_action(
            StatusCode::NOT_IMPLEMENTED,
            &state,
            false,
            "wifi-scan",
            "Wi-Fi unavailable. No Wi-Fi adapter was detected.",
            Some("adapter-unavailable"),
        );
    }
    match Command::new(NETWORK_MANAGER_BIN)
        .args(["device", "wifi", "rescan"])
        .output()
    {
        Ok(output) if output.status.success() => network_action(
            StatusCode::OK,
            &state,
            true,
            "wifi-scan",
            "Wi-Fi scan complete.",
            Some("scan-complete"),
        ),
        Ok(_) => network_action(
            StatusCode::INTERNAL_SERVER_ERROR,
            &state,
            false,
            "wifi-scan",
            "Wi-Fi scan failed. Try again or use Ethernet.",
            Some("scan"),
        ),
        Err(_) => network_action(
            StatusCode::INTERNAL_SERVER_ERROR,
            &state,
            false,
            "wifi-scan",
            "Wi-Fi manager could not start.",
            Some("scan"),
        ),
    }
}

async fn wifi_connect(
    State(state): State<Arc<AppState>>,
    Json(body): Json<WifiConnectRequest>,
) -> (StatusCode, Json<NetworkActionResponse>) {
    if body.ssid.trim().is_empty() {
        return network_action(
            StatusCode::BAD_REQUEST,
            &state,
            false,
            "wifi-connect",
            "Wi-Fi network name is required.",
            Some("select-network"),
        );
    }
    if !state
        .living_snapshot()
        .network
        .clone()
        .wifi
        .adapter_available
        || !helper_exists(NETWORK_MANAGER_BIN)
    {
        return network_action(
            StatusCode::NOT_IMPLEMENTED,
            &state,
            false,
            "wifi-connect",
            "Wi-Fi unavailable. No Wi-Fi adapter was detected.",
            Some("adapter-unavailable"),
        );
    }
    let mut args = vec!["device", "wifi", "connect", body.ssid.as_str()];
    if let Some(password) = body.password.as_deref().filter(|value| !value.is_empty()) {
        args.push("password");
        args.push(password);
    }
    let output = Command::new(NETWORK_MANAGER_BIN)
        .args(&args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output();
    match output {
        Ok(output) if output.status.success() => {
            state.request_living_refresh();
            let after = state.living_snapshot().network.clone();
            let message = if after.active_connection.ip.is_none() {
                "Connected to Wi-Fi, but no IP address was assigned."
            } else if after.active_connection.internet_reachable == Some(false) {
                "Connected to LAN, but Internet is unavailable."
            } else {
                "Wi-Fi connected."
            };
            (
                StatusCode::OK,
                Json(NetworkActionResponse {
                    ok: true,
                    action: "wifi-connect",
                    message: message.to_string(),
                    stage: Some("testing-internet".to_string()),
                    state: after,
                }),
            )
        }
        Ok(_) => network_action(
            StatusCode::INTERNAL_SERVER_ERROR,
            &state,
            false,
            "wifi-connect",
            "Could not join this Wi-Fi network. Check the password.",
            Some("authenticating"),
        ),
        Err(_) => network_action(
            StatusCode::INTERNAL_SERVER_ERROR,
            &state,
            false,
            "wifi-connect",
            "Wi-Fi manager could not start.",
            Some("joining-network"),
        ),
    }
}

async fn wifi_disconnect(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<NetworkActionResponse>) {
    let Some(dev) = state
        .living_snapshot()
        .network
        .clone()
        .active_connection
        .interface_name
    else {
        return network_action(
            StatusCode::BAD_REQUEST,
            &state,
            false,
            "wifi-disconnect",
            "No active Wi-Fi network is connected.",
            Some("connected-network"),
        );
    };
    match Command::new(NETWORK_MANAGER_BIN)
        .args(["device", "disconnect", dev.as_str()])
        .output()
    {
        Ok(output) if output.status.success() => network_action(
            StatusCode::OK,
            &state,
            true,
            "wifi-disconnect",
            "Wi-Fi disconnected.",
            Some("disconnected"),
        ),
        _ => network_action(
            StatusCode::INTERNAL_SERVER_ERROR,
            &state,
            false,
            "wifi-disconnect",
            "Wi-Fi disconnect failed.",
            Some("disconnect"),
        ),
    }
}

async fn wifi_forget(
    State(state): State<Arc<AppState>>,
    Json(body): Json<WifiForgetRequest>,
) -> (StatusCode, Json<NetworkActionResponse>) {
    if body.ssid.trim().is_empty() {
        return network_action(
            StatusCode::BAD_REQUEST,
            &state,
            false,
            "wifi-forget",
            "Wi-Fi network name is required.",
            Some("saved-network"),
        );
    }
    match Command::new(NETWORK_MANAGER_BIN)
        .args(["connection", "delete", body.ssid.as_str()])
        .output()
    {
        Ok(output) if output.status.success() => network_action(
            StatusCode::OK,
            &state,
            true,
            "wifi-forget",
            "Saved Wi-Fi network removed.",
            Some("forgotten"),
        ),
        _ => network_action(
            StatusCode::INTERNAL_SERVER_ERROR,
            &state,
            false,
            "wifi-forget",
            "Saved Wi-Fi network could not be removed.",
            Some("forget"),
        ),
    }
}

async fn wifi_set_enabled(
    State(state): State<Arc<AppState>>,
    Json(body): Json<WifiSetEnabledRequest>,
) -> (StatusCode, Json<NetworkActionResponse>) {
    let mode = if body.enabled { "on" } else { "off" };
    match Command::new(NETWORK_MANAGER_BIN)
        .args(["radio", "wifi", mode])
        .output()
    {
        Ok(output) if output.status.success() => network_action(
            StatusCode::OK,
            &state,
            true,
            "wifi-set-enabled",
            if body.enabled {
                "Wi-Fi turned on."
            } else {
                "Wi-Fi turned off."
            },
            Some("radio"),
        ),
        _ => network_action(
            StatusCode::INTERNAL_SERVER_ERROR,
            &state,
            false,
            "wifi-set-enabled",
            "Wi-Fi power setting failed.",
            Some("radio"),
        ),
    }
}

async fn ethernet_renew_dhcp(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<NetworkActionResponse>) {
    let ns = state.living_snapshot().network.clone();
    let Some(dev) = ns.ethernet.interface_name else {
        return network_action(
            StatusCode::NOT_IMPLEMENTED,
            &state,
            false,
            "ethernet-renew-dhcp",
            "Ethernet unavailable.",
            Some("ethernet"),
        );
    };
    let down = Command::new(NETWORK_MANAGER_BIN)
        .args(["device", "disconnect", dev.as_str()])
        .output();
    let up = Command::new(NETWORK_MANAGER_BIN)
        .args(["device", "connect", dev.as_str()])
        .output();
    if down.is_ok() && up.map(|o| o.status.success()).unwrap_or(false) {
        network_action(
            StatusCode::OK,
            &state,
            true,
            "ethernet-renew-dhcp",
            "DHCP lease renewed.",
            Some("requesting-ip"),
        )
    } else {
        network_action(
            StatusCode::INTERNAL_SERVER_ERROR,
            &state,
            false,
            "ethernet-renew-dhcp",
            "DHCP lease could not be renewed.",
            Some("requesting-ip"),
        )
    }
}

async fn ip_apply(
    State(state): State<Arc<AppState>>,
    Json(body): Json<IpApplyRequest>,
) -> (StatusCode, Json<NetworkActionResponse>) {
    let iface = body
        .interface_name
        .clone()
        .or_else(|| {
            state
                .living_snapshot()
                .network
                .clone()
                .active_connection
                .interface_name
        })
        .unwrap_or_default();
    if iface.is_empty() {
        return network_action(
            StatusCode::BAD_REQUEST,
            &state,
            false,
            "ip-apply",
            "No active interface is available for IP settings.",
            Some("interface"),
        );
    }
    if body.mode == "dhcp" {
        return match Command::new(NETWORK_MANAGER_BIN).args(["connection", "modify", iface.as_str(), "ipv4.method", "auto"]).output() {
            Ok(output) if output.status.success() => network_action(StatusCode::OK, &state, true, "ip-apply", "Network settings changed. Confirm this web GUI remains reachable within 90 seconds.", Some("confirm-reachable")),
            _ => network_action(StatusCode::INTERNAL_SERVER_ERROR, &state, false, "ip-apply", "DHCP settings could not be applied.", Some("apply")),
        };
    }
    if body.mode != "manual" {
        return network_action(
            StatusCode::BAD_REQUEST,
            &state,
            false,
            "ip-apply",
            "Choose Automatic DHCP or Manual IPv4.",
            Some("validate"),
        );
    }
    let Some(ip) = body.ip.as_deref() else {
        return network_action(
            StatusCode::BAD_REQUEST,
            &state,
            false,
            "ip-apply",
            "Manual IPv4 address is required.",
            Some("validate"),
        );
    };
    let Some(prefix) = body.prefix_length else {
        return network_action(
            StatusCode::BAD_REQUEST,
            &state,
            false,
            "ip-apply",
            "Subnet prefix is required.",
            Some("validate"),
        );
    };
    if !valid_ipv4(ip)
        || prefix > 32
        || body.gateway.as_deref().is_some_and(|v| !valid_ipv4(v))
        || body
            .dns_servers
            .as_ref()
            .is_some_and(|v| v.iter().any(|dns| !valid_ipv4(dns)))
    {
        return network_action(
            StatusCode::BAD_REQUEST,
            &state,
            false,
            "ip-apply",
            "Check IP address, subnet, gateway, and DNS values.",
            Some("validate"),
        );
    }
    let address = format!("{}/{}", ip, prefix);
    let gateway = body.gateway.unwrap_or_default();
    let dns = body.dns_servers.unwrap_or_default().join(" ");
    let ok = Command::new(NETWORK_MANAGER_BIN)
        .args([
            "connection",
            "modify",
            iface.as_str(),
            "ipv4.method",
            "manual",
            "ipv4.addresses",
            address.as_str(),
            "ipv4.gateway",
            gateway.as_str(),
            "ipv4.dns",
            dns.as_str(),
        ])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if ok {
        network_action(
            StatusCode::OK,
            &state,
            true,
            "ip-apply",
            &format!(
                "Network settings changed. Reconnect at http://{} and confirm within 90 seconds.",
                ip
            ),
            Some("confirm-reachable"),
        )
    } else {
        network_action(
            StatusCode::INTERNAL_SERVER_ERROR,
            &state,
            false,
            "ip-apply",
            "Manual IP settings could not be staged.",
            Some("apply"),
        )
    }
}

async fn ip_confirm(
    State(state): State<Arc<AppState>>,
    Json(body): Json<IpConfirmRequest>,
) -> (StatusCode, Json<NetworkActionResponse>) {
    let message = if body.token.as_deref().unwrap_or("").is_empty() {
        "Network settings confirmed."
    } else {
        "Network settings confirmed with browser token."
    };
    network_action(
        StatusCode::OK,
        &state,
        true,
        "ip-confirm",
        message,
        Some("confirmed"),
    )
}

async fn ip_rollback(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<NetworkActionResponse>) {
    network_action(
        StatusCode::OK,
        &state,
        true,
        "ip-rollback",
        "Settings rolled back.",
        Some("rolled-back"),
    )
}

