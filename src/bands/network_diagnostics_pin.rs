async fn speed_test_run() -> (StatusCode, Json<SpeedTestResponse>) {
    let result = run_download_speed_test();
    let status = if result.ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (status, Json(result))
}

async fn diagnostics_run(
    State(state): State<Arc<AppState>>,
    Json(body): Json<DiagnosticsRequest>,
) -> (StatusCode, Json<DiagnosticsResponse>) {
    let requested = body.tests.unwrap_or_else(|| {
        vec![
            "gateway".into(),
            "dns".into(),
            "internet".into(),
            "game-folders".into(),
            "lan-ai".into(),
        ]
    });
    let ns = state.living_snapshot().network.clone();
    let mut results = Vec::new();
    for test in requested {
        match test.as_str() {
            "gateway" => results.push(diag(
                "Gateway",
                ns.active_connection.gateway.is_some(),
                if ns.active_connection.gateway.is_some() {
                    "Gateway reachable"
                } else {
                    "Gateway unreachable. Check cable, Wi-Fi, or DHCP."
                },
            )),
            "dns" => results.push(diag(
                "DNS",
                !ns.active_connection.dns_servers.is_empty(),
                if ns.active_connection.dns_servers.is_empty() {
                    "DNS failed. Internet names may not resolve."
                } else {
                    "DNS working"
                },
            )),
            "internet" => results.push(diag(
                "Internet",
                ns.active_connection.internet_reachable == Some(true),
                if ns.active_connection.internet_reachable == Some(true) {
                    "Internet reachable"
                } else {
                    "Internet unavailable"
                },
            )),
            "game-folders" => results.push(diag(
                "Game folders",
                ns.services.samba.state == "available",
                if ns.services.samba.state == "available" {
                    "Game folders available"
                } else {
                    "Game folders unavailable. Samba may be stopped."
                },
            )),
            "lan-ai" => results.push(diag(
                "LAN AI",
                ns.services.lan_inference.state == "available",
                if ns.services.lan_inference.state == "available" {
                    "LAN AI available"
                } else {
                    "LAN AI disabled"
                },
            )),
            _ => results.push(diag("Unknown", false, "Unknown diagnostic.")),
        }
    }
    let ok = results.iter().all(|r| r.ok || r.name == "LAN AI");
    (
        StatusCode::OK,
        Json(DiagnosticsResponse {
            ok,
            action: "network-diagnostics",
            results,
            state: ns,
        }),
    )
}

fn network_action(
    status: StatusCode,
    state: &AppState,
    ok: bool,
    action: &'static str,
    message: &str,
    stage: Option<&str>,
) -> (StatusCode, Json<NetworkActionResponse>) {
    if ok { state.request_living_refresh(); }
    (
        status,
        Json(NetworkActionResponse {
            ok,
            action,
            message: message.to_string(),
            stage: stage.map(str::to_string),
            state: state.living_snapshot().network.clone(),
        }),
    )
}

fn diag(name: &str, ok: bool, message: &str) -> DiagnosticResult {
    DiagnosticResult {
        name: name.to_string(),
        ok,
        message: message.to_string(),
        detail: None,
    }
}

fn valid_ipv4(value: &str) -> bool {
    value.parse::<Ipv4Addr>().is_ok()
}
