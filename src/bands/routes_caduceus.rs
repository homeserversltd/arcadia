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

fn caduceus_post_json_with_timeout(
    path: &str,
    body: &str,
    timeout_seconds: &str,
) -> Result<serde_json::Value, &'static str> {
    let url = caduceus_proxy_url(path);
    let output = Command::new("curl")
        .args([
            "-sS",
            "--max-time",
            timeout_seconds,
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

fn caduceus_post_json(path: &str, body: &str) -> Result<serde_json::Value, &'static str> {
    caduceus_post_json_with_timeout(path, body, "300")
}

/// Reflect one bounded Arcadia control result through Caduceus. Caduceus owns
/// profile admission, redaction, schema completion, and the sole channel write.
/// Telemetry cannot alter the action result it observes.
fn caduceus_hyalos_reflect_action(action: &str, path: &str, ok: bool) -> bool {
    let message = if ok {
        format!("Arcadia control action completed: {action}")
    } else {
        format!("Arcadia control action failed: {action}")
    };
    let body = serde_json::json!({
        "organ": "arcadia",
        "kind": "control-action",
        "level": if ok { "info" } else { "warn" },
        "ok": ok,
        "message": message,
        "attributes_redacted": {
            "action": action,
            "caduceus_path": path,
        },
    });
    let Ok(body) = serde_json::to_string(&body) else {
        return false;
    };
    caduceus_post_json_with_timeout("/api/v1/hyalos/reflect", &body, "2").is_ok()
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

async fn caduceus_gui_update_now_proxy_route() -> impl IntoResponse {
    match caduceus_post_json("/api/v1/gui/update/now", "{}") {
        Ok(value) => {
            let ok = value.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
            let status = if ok {
                StatusCode::OK
            } else {
                StatusCode::BAD_GATEWAY
            };
            (status, Json(value)).into_response()
        }
        Err(signal) => caduceus_proxy_error("/api/v1/gui/update/now", signal),
    }
}

async fn caduceus_local_ai_runtime_status_proxy_route() -> impl IntoResponse {
    caduceus_json_proxy("/api/v1/local-ai/runtime/status").await
}

async fn caduceus_local_ai_runtime_check_proxy_route() -> impl IntoResponse {
    match caduceus_post_json("/api/v1/local-ai/runtime/check", "{}") {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(signal) => caduceus_proxy_error("/api/v1/local-ai/runtime/check", signal),
    }
}

async fn caduceus_local_ai_runtime_update_proxy_route() -> impl IntoResponse {
    match caduceus_post_json("/api/v1/local-ai/runtime/update", "{}") {
        Ok(value) => {
            let ok = value.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
            let status = if ok {
                StatusCode::OK
            } else {
                StatusCode::BAD_GATEWAY
            };
            (status, Json(value)).into_response()
        }
        Err(signal) => caduceus_proxy_error("/api/v1/local-ai/runtime/update", signal),
    }
}

async fn caduceus_profile_module_toggle_proxy_route(
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let rendered =
        serde_json::to_string(&body).unwrap_or_else(|_| "{\"module_id\":\"\",\"enabled\":false}".to_string());
    match caduceus_post_json("/api/v1/profile/module/toggle", &rendered) {
        Ok(value) => {
            let ok = value.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
            let status = if ok {
                StatusCode::OK
            } else {
                StatusCode::BAD_REQUEST
            };
            (status, Json(value)).into_response()
        }
        Err(signal) => caduceus_proxy_error("/api/v1/profile/module/toggle", signal),
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
            let _ = caduceus_hyalos_reflect_action(action, path, ok);
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
        Err(signal) => {
            let _ = caduceus_hyalos_reflect_action(action, path, false);
            console_action_error(
                StatusCode::BAD_GATEWAY,
                action,
                "caduceus-http",
                signal,
            )
        }
    }
}

#[cfg(test)]
mod hyalos_emitter_tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::{Shutdown, TcpListener},
        sync::{Arc, Mutex, OnceLock},
        thread,
    };

    static CADUCEUS_ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    #[test]
    fn hyalos_emitter_posts_a_bounded_arcadia_reflection_without_local_file_writes() {
        let _guard = CADUCEUS_ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").expect("mock caduceus bind");
        let port = listener.local_addr().expect("mock caduceus address").port();
        let captured = Arc::new(Mutex::new(String::new()));
        let captured_thread = captured.clone();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("Arcadia reflection request");
            let mut buf = [0u8; 8192];
            let read = stream.read(&mut buf).expect("reflection request bytes");
            *captured_thread.lock().unwrap() = String::from_utf8_lossy(&buf[..read]).to_string();
            let response = r#"{"ok":true,"firstMissingSignal":"none"}"#;
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}", response.len()).unwrap();
            let _ = stream.shutdown(Shutdown::Write);
        });

        std::env::set_var("CADUCEUS_HTTP_BASE", format!("http://127.0.0.1:{port}"));
        assert!(caduceus_hyalos_reflect_action("check-updates", "/api/v1/update/check", true));
        handle.join().unwrap();
        std::env::remove_var("CADUCEUS_HTTP_BASE");

        let request = captured.lock().unwrap().clone();
        assert!(request.starts_with("POST /api/v1/hyalos/reflect HTTP/1.1"), "{request}");
        let body = request.split("\r\n\r\n").nth(1).unwrap_or("");
        let body: serde_json::Value = serde_json::from_str(body).expect("reflection JSON");
        assert_eq!(body["organ"], "arcadia");
        assert_eq!(body["kind"], "control-action");
        assert_eq!(body["level"], "info");
        assert_eq!(body["ok"], true);
        assert_eq!(body["attributes_redacted"]["action"], "check-updates");
        assert_eq!(body["attributes_redacted"]["caduceus_path"], "/api/v1/update/check");

        let source = include_str!("routes_caduceus.rs");
        let direct_open = ["OpenOptions", "::new"].concat();
        let direct_write = ["fs", "::write("].concat();
        assert!(!source.contains(&direct_open));
        assert!(!source.contains(&direct_write));
    }

    #[test]
    fn hyalos_failure_does_not_change_a_successful_control_action() {
        let _guard = CADUCEUS_ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").expect("mock caduceus bind");
        let port = listener.local_addr().expect("mock caduceus address").port();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("control action request");
            let mut buf = [0u8; 8192];
            let _ = stream.read(&mut buf).expect("control action bytes");
            let response = r#"{"ok":true,"body":"complete","firstMissingSignal":"none"}"#;
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}", response.len()).unwrap();
            let _ = stream.shutdown(Shutdown::Write);
        });

        std::env::set_var("CADUCEUS_HTTP_BASE", format!("http://127.0.0.1:{port}"));
        let (status, Json(response)) = run_caduceus_http_mutation(
            "check-updates",
            "/api/v1/update/check",
            "check complete",
            "check failed",
        );
        handle.join().unwrap();
        std::env::remove_var("CADUCEUS_HTTP_BASE");

        assert_eq!(status, StatusCode::OK);
        assert!(response.ok);
        assert_eq!(response.action, "check-updates");
    }
}