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
    let ns = network_state(&state);
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
    (
        status,
        Json(NetworkActionResponse {
            ok,
            action,
            message: message.to_string(),
            stage: stage.map(str::to_string),
            state: network_state(state),
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

async fn pre_unlock(
    _body: Json<GuiPinUnlockRequest>,
) -> (StatusCode, Json<GuiPinActionResponse>) {
    gui_pin_response(
        StatusCode::GONE,
        false,
        gui_pin_required(),
        "verify-pin",
        false,
        "GUI PIN attendance is now verified by Caduceus.",
    )
}

async fn set_gui_pin_access(
    Json(body): Json<GuiPinAccessRequest>,
) -> (StatusCode, Json<GuiPinActionResponse>) {
    run_gui_pin_helper(
        GUI_PIN_ACCESS_HELPER,
        if body.pin_required {
            "require-gui-pin"
        } else {
            "disable-gui-pin"
        },
        &[if body.pin_required {
            "required"
        } else {
            "disabled"
        }],
        if body.pin_required {
            "GUI PIN is now required before Arcadia opens."
        } else {
            "GUI PIN gate is disabled; Arcadia opens directly."
        },
        "GUI PIN access setting failed.",
    )
}

async fn change_gui_pin(
    Json(body): Json<PinChangeRequest>,
) -> (StatusCode, Json<GuiPinActionResponse>) {
    if body.current_pin.is_empty() || body.new_pin.is_empty() {
        return gui_pin_response(
            StatusCode::BAD_REQUEST,
            false,
            gui_pin_required(),
            "change-pin",
            helper_exists(GUI_PIN_CHANGE_HELPER),
            "Current PIN and new PIN are required.",
        );
    }
    if body.new_pin.len() < 4 {
        return gui_pin_response(
            StatusCode::BAD_REQUEST,
            false,
            gui_pin_required(),
            "change-pin",
            helper_exists(GUI_PIN_CHANGE_HELPER),
            "New PIN is too short.",
        );
    }

    run_gui_pin_helper(
        GUI_PIN_CHANGE_HELPER,
        "change-pin",
        &[body.current_pin.as_str(), body.new_pin.as_str()],
        "GUI PIN changed.",
        "GUI PIN change failed.",
    )
}

async fn reset_gui_pin_default(
    Json(body): Json<PinResetRequest>,
) -> (StatusCode, Json<GuiPinActionResponse>) {
    if body.confirm != "RESET" {
        return gui_pin_response(
            StatusCode::BAD_REQUEST,
            false,
            gui_pin_required(),
            "reset-default-pin",
            helper_exists(GUI_PIN_RESET_HELPER),
            "Type RESET to restore the default GUI PIN.",
        );
    }

    run_gui_pin_helper(
        GUI_PIN_RESET_HELPER,
        "reset-default-pin",
        &[],
        "GUI PIN reset to the appliance default.",
        "GUI PIN reset failed.",
    )
}

fn run_gui_pin_helper(
    helper: &'static str,
    action: &'static str,
    secret_lines: &[&str],
    success_message: &str,
    failure_message: &str,
) -> (StatusCode, Json<GuiPinActionResponse>) {
    if !helper_exists(helper) {
        return gui_pin_response(
            StatusCode::NOT_IMPLEMENTED,
            false,
            gui_pin_required(),
            action,
            false,
            "GUI PIN helper is not installed on this unit yet.",
        );
    }

    let mut child = match Command::new(helper)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => {
            return gui_pin_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                false,
                gui_pin_required(),
                action,
                true,
                "Failed to start GUI PIN helper.",
            )
        }
    };

    if let Some(mut stdin) = child.stdin.take() {
        for line in secret_lines {
            let _ = stdin.write_all(line.as_bytes());
            let _ = stdin.write_all(b"\n");
        }
    }

    let ok = child.wait().map(|status| status.success()).unwrap_or(false);
    gui_pin_response(
        if ok {
            StatusCode::OK
        } else {
            StatusCode::FORBIDDEN
        },
        ok,
        gui_pin_required(),
        action,
        true,
        if ok { success_message } else { failure_message },
    )
}

fn gui_pin_response(
    status: StatusCode,
    ok: bool,
    pin_required: bool,
    action: &'static str,
    helper_present: bool,
    message: &str,
) -> (StatusCode, Json<GuiPinActionResponse>) {
    (
        status,
        Json(GuiPinActionResponse {
            ok,
            pin_required,
            action,
            helper_present,
            message: message.to_string(),
        }),
    )
}
