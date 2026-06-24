fn caduceus_http_base() -> String {
    env::var("CADUCEUS_HTTP_BASE").unwrap_or_else(|_| CADUCEUS_HTTP_BASE.to_string())
}

fn caduceus_proxy_url(path: &str) -> String {
    format!(
        "{}{}",
        caduceus_http_base().trim_end_matches('/'),
        path
    )
}

fn caduceus_fetch_json(path: &str) -> Result<serde_json::Value, &'static str> {
    let url = caduceus_proxy_url(path);
    let text = command_stdout("curl", &["-fsS", "--max-time", "5", &url])
        .ok_or("caduceus-http-unreachable")?;
    serde_json::from_str(&text).map_err(|_| "caduceus-http-invalid-json")
}

fn caduceus_post_json(path: &str, body: &str) -> Result<serde_json::Value, &'static str> {
    let url = caduceus_proxy_url(path);
    let output = Command::new("curl")
        .args([
            "-sS",
            "--max-time",
            "300",
            "-X",
            "POST",
            "-H",
            "Content-Type: application/json",
            "-d",
            body,
            &url,
        ])
        .output()
        .map_err(|_| "caduceus-http-unreachable")?;
    if output.stdout.is_empty() {
        return Err("caduceus-http-empty-response");
    }
    serde_json::from_slice(&output.stdout).map_err(|_| "caduceus-http-invalid-json")
}

fn caduceus_proxy_error(path: &str, signal: &'static str) -> axum::response::Response {
    (
        StatusCode::BAD_GATEWAY,
        Json(serde_json::json!({
            "schema": "arcadia.caduceus.proxy.error.v1",
            "ok": false,
            "path": path,
            "first_missing_signal": signal,
        })),
    )
        .into_response()
}

async fn caduceus_json_proxy(path: &'static str) -> impl IntoResponse {
    match caduceus_fetch_json(path) {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(signal) => caduceus_proxy_error(path, signal),
    }
}

async fn caduceus_health_proxy_route() -> impl IntoResponse {
    caduceus_json_proxy("/health").await
}

async fn caduceus_identity_proxy_route() -> impl IntoResponse {
    caduceus_json_proxy("/api/v1/identity").await
}

async fn caduceus_profile_proxy_route() -> impl IntoResponse {
    caduceus_json_proxy("/api/v1/profile").await
}

async fn caduceus_health_api_proxy_route() -> impl IntoResponse {
    caduceus_json_proxy("/api/v1/health").await
}

async fn caduceus_update_status_proxy_route() -> impl IntoResponse {
    caduceus_json_proxy("/api/v1/update/status").await
}

async fn caduceus_update_now_proxy_route() -> impl IntoResponse {
    match caduceus_post_json("/api/v1/update/now", "{}") {
        Ok(value) => {
            let ok = value.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
            let status = if ok {
                StatusCode::OK
            } else {
                StatusCode::BAD_GATEWAY
            };
            (status, Json(value)).into_response()
        }
        Err(signal) => caduceus_proxy_error("/api/v1/update/now", signal),
    }
}

async fn caduceus_update_check_proxy_route() -> impl IntoResponse {
    match caduceus_post_json("/api/v1/update/check", "{}") {
        Ok(value) => {
            let ok = value.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
            let status = if ok {
                StatusCode::OK
            } else {
                StatusCode::BAD_GATEWAY
            };
            (status, Json(value)).into_response()
        }
        Err(signal) => caduceus_proxy_error("/api/v1/update/check", signal),
    }
}

async fn caduceus_sync_status_proxy_route() -> impl IntoResponse {
    caduceus_json_proxy("/api/v1/sync/status").await
}

async fn caduceus_sync_now_proxy_route() -> impl IntoResponse {
    match caduceus_post_json("/api/v1/sync/now", "{}") {
        Ok(value) => {
            let ok = value.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
            let status = if ok {
                StatusCode::OK
            } else {
                StatusCode::BAD_GATEWAY
            };
            (status, Json(value)).into_response()
        }
        Err(signal) => caduceus_proxy_error("/api/v1/sync/now", signal),
    }
}

async fn caduceus_receipts_latest_proxy_route() -> impl IntoResponse {
    caduceus_json_proxy("/api/v1/receipts/latest").await
}

async fn caduceus_receipts_ledger_proxy_route(
    Query(query): Query<HarmoniaLedgerQuery>,
) -> impl IntoResponse {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(10).clamp(1, 25);
    let path = format!("/api/v1/receipts/ledger?page={page}&per_page={per_page}");
    match caduceus_fetch_json(&path) {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(signal) => caduceus_proxy_error(&path, signal),
    }
}

async fn caduceus_update_service_status_proxy_route() -> impl IntoResponse {
    caduceus_json_proxy("/api/v1/update/service/status").await
}

async fn caduceus_update_service_toggle_proxy_route(
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let payload = if body.get("state").is_some() {
        body
    } else {
        serde_json::json!({ "state": "on" })
    };
    let rendered = serde_json::to_string(&payload).unwrap_or_else(|_| "{\"state\":\"on\"}".to_string());
    match caduceus_post_json("/api/v1/update/service/toggle", &rendered) {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(signal) => caduceus_proxy_error("/api/v1/update/service/toggle", signal),
    }
}

fn run_caduceus_http_mutation(
    action: &'static str,
    path: &'static str,
    success_message: &str,
    failure_message: &str,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    match caduceus_post_json(path, "{}") {
        Ok(value) => {
            let ok = value.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
            let stdout = value
                .get("body")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let signal = value
                .get("firstMissingSignal")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            (
                if ok {
                    StatusCode::OK
                } else {
                    StatusCode::INTERNAL_SERVER_ERROR
                },
                Json(ConsoleActionResponse {
                    ok,
                    action,
                    command: "caduceus-http",
                    exit_code: value
                        .get("exitCode")
                        .and_then(|v| v.as_i64())
                        .map(|v| v as i32),
                    message: if ok {
                        success_message.to_string()
                    } else {
                        failure_message.to_string()
                    },
                    stdout,
                    stderr: signal,
                }),
            )
        }
        Err(signal) => console_action_error(
            StatusCode::BAD_GATEWAY,
            action,
            "caduceus-http",
            signal,
        ),
    }
}