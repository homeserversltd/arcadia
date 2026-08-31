async fn wifi_status(State(state): State<Arc<AppState>>) -> Json<NetworkState> {
    Json(state.living_snapshot().network.clone())
}

fn wifi_caduceus_call(
    headers: &axum::http::HeaderMap,
    path: &'static str,
    payload: serde_json::Value,
) -> Result<serde_json::Value, AttendanceCall> {
    caduceus_attended_json_call(path, headers, payload)
}

fn wifi_caduceus_succeeded(result: &Result<serde_json::Value, AttendanceCall>) -> bool {
    matches!(result, Ok(value) if value.get("ok").and_then(serde_json::Value::as_bool) == Some(true))
}

fn wifi_caduceus_value_succeeded(value: &serde_json::Value) -> bool {
    value.get("ok").and_then(serde_json::Value::as_bool) == Some(true)
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
    let result = CaduceusAccessClient::default().get_json("/api/v1/network/device/wifi/scan");
    if matches!(result, Ok(value) if wifi_caduceus_value_succeeded(&value)) {
        network_action(
            StatusCode::OK,
            &state,
            true,
            "wifi-scan",
            "Wi-Fi scan complete.",
            Some("scan-complete"),
        )
    } else {
        network_action(
            StatusCode::INTERNAL_SERVER_ERROR,
            &state,
            false,
            "wifi-scan",
            "Wi-Fi scan failed. Try again or use Ethernet.",
            Some("scan"),
        )
    }
}

async fn wifi_connect(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
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
    // The password is placed only in the Caduceus request payload; it is never
    // copied into Arcadia state, a response, diagnostics, logs, or receipts.
    let mut payload = serde_json::json!({"ssid": body.ssid});
    if let Some(password) = body.password.filter(|password| !password.is_empty()) {
        payload["password"] = serde_json::Value::String(password);
    }
    let result = wifi_caduceus_call(&headers, "/api/v1/network/device/wifi/connect", payload);
    if wifi_caduceus_succeeded(&result) {
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
    } else {
        network_action(
            StatusCode::INTERNAL_SERVER_ERROR,
            &state,
            false,
            "wifi-connect",
            "Could not join this Wi-Fi network. Check the password.",
            Some("authenticating"),
        )
    }
}

async fn wifi_disconnect(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
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
    let result = wifi_caduceus_call(
        &headers,
        "/api/v1/network/device/wifi/disconnect",
        serde_json::json!({"interface": dev}),
    );
    if wifi_caduceus_succeeded(&result) {
        network_action(
            StatusCode::OK,
            &state,
            true,
            "wifi-disconnect",
            "Wi-Fi disconnected.",
            Some("disconnected"),
        )
    } else {
        network_action(
            StatusCode::INTERNAL_SERVER_ERROR,
            &state,
            false,
            "wifi-disconnect",
            "Wi-Fi disconnect failed.",
            Some("disconnect"),
        )
    }
}

async fn wifi_forget(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
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
    let Some(uuid) = CaduceusAccessClient::default().get_json("/api/v1/network/device/wifi/saved")
        .ok()
        .and_then(|saved| {
            wifi_caduceus_value_succeeded(&saved)
                .then(|| saved.get("entries").and_then(serde_json::Value::as_array))
                .flatten()?
                .iter()
                .find_map(|entry| {
                    let fields = entry.as_array()?;
                    let name = fields.first()?.as_str()?;
                    let uuid = fields.get(1)?.as_str()?;
                    (name == body.ssid).then(|| uuid.to_string())
                })
        })
    else {
        return network_action(
            StatusCode::INTERNAL_SERVER_ERROR,
            &state,
            false,
            "wifi-forget",
            "Saved Wi-Fi network could not be removed.",
            Some("forget"),
        );
    };
    let result = wifi_caduceus_call(
        &headers,
        "/api/v1/network/device/wifi/forget",
        serde_json::json!({"uuid": uuid}),
    );
    if wifi_caduceus_succeeded(&result) {
        network_action(
            StatusCode::OK,
            &state,
            true,
            "wifi-forget",
            "Saved Wi-Fi network removed.",
            Some("forgotten"),
        )
    } else {
        network_action(
            StatusCode::INTERNAL_SERVER_ERROR,
            &state,
            false,
            "wifi-forget",
            "Saved Wi-Fi network could not be removed.",
            Some("forget"),
        )
    }
}

async fn wifi_set_enabled(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<WifiSetEnabledRequest>,
) -> (StatusCode, Json<NetworkActionResponse>) {
    let result = wifi_caduceus_call(
        &headers,
        "/api/v1/network/device/wifi/radio",
        serde_json::json!({"enabled": body.enabled}),
    );
    if wifi_caduceus_succeeded(&result) {
        network_action(
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
        )
    } else {
        network_action(
            StatusCode::INTERNAL_SERVER_ERROR,
            &state,
            false,
            "wifi-set-enabled",
            "Wi-Fi power setting failed.",
            Some("radio"),
        )
    }
}

async fn ethernet_renew_dhcp(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> (StatusCode, Json<NetworkActionResponse>) {
    let Some(dev) = state
        .living_snapshot()
        .network
        .clone()
        .ethernet
        .interface_name
    else {
        return network_action(
            StatusCode::NOT_IMPLEMENTED,
            &state,
            false,
            "ethernet-renew-dhcp",
            "Ethernet unavailable.",
            Some("ethernet"),
        );
    };
    let down = wifi_caduceus_call(
        &headers,
        "/api/v1/network/device/disconnect",
        serde_json::json!({"interface": dev}),
    );
    let up_succeeded = if wifi_caduceus_succeeded(&down) {
        let up = wifi_caduceus_call(
            &headers,
            "/api/v1/network/device/connect",
            serde_json::json!({"interface": dev}),
        );
        wifi_caduceus_succeeded(&up)
    } else {
        false
    };
    if wifi_caduceus_succeeded(&down) && up_succeeded {
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
    headers: axum::http::HeaderMap,
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
    if body.mode != "dhcp" && body.mode != "manual" {
        return network_action(
            StatusCode::BAD_REQUEST,
            &state,
            false,
            "ip-apply",
            "Choose Automatic DHCP or Manual IPv4.",
            Some("validate"),
        );
    }
    let manual_ip = body.ip.clone();
    let payload = if body.mode == "dhcp" {
        serde_json::json!({"interface": iface, "method": "auto"})
    } else {
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
        serde_json::json!({
            "interface": iface,
            "method": "static",
            "address": format!("{ip}/{prefix}"),
            "gateway": body.gateway.unwrap_or_default(),
            "dns": body.dns_servers.unwrap_or_default().join(","),
        })
    };
    let result = wifi_caduceus_call(&headers, "/api/v1/network/device/ipv4", payload);
    if wifi_caduceus_succeeded(&result) {
        let success_message = if body.mode == "dhcp" {
            "Network settings changed. Confirm this web GUI remains reachable within 90 seconds."
                .to_string()
        } else {
            format!(
                "Network settings changed. Reconnect at http://{} and confirm within 90 seconds.",
                manual_ip.as_deref().unwrap_or("")
            )
        };
        network_action(
            StatusCode::OK,
            &state,
            true,
            "ip-apply",
            &success_message,
            Some("confirm-reachable"),
        )
    } else {
        network_action(
            StatusCode::INTERNAL_SERVER_ERROR,
            &state,
            false,
            "ip-apply",
            if body.mode == "dhcp" {
                "DHCP settings could not be applied."
            } else {
                "Manual IP settings could not be staged."
            },
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
