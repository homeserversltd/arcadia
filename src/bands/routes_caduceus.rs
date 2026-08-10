fn caduceus_http_base() -> String {
    env::var("CADUCEUS_HTTP_BASE").unwrap_or_else(|_| CADUCEUS_HTTP_BASE.to_string())
}

fn caduceus_proxy_url(path: &str) -> String {
    format!("{}{}", caduceus_http_base().trim_end_matches('/'), path)
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

const ARCADIA_DEBUG_MAX_DEPTH: usize = 4;
const ARCADIA_DEBUG_MAX_ITEMS: usize = 32;
const ARCADIA_DEBUG_MAX_ARRAY_ITEMS: usize = 16;
const ARCADIA_DEBUG_MAX_TEXT: usize = 512;

fn arcadia_debug_kind_is_safe(kind: &str) -> bool {
    !kind.is_empty()
        && kind.len() <= 48
        && kind
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !kind.starts_with('-')
        && !kind.ends_with('-')
        && !kind.contains("--")
}

fn arcadia_debug_identifier_is_safe(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'_' | b'-'))
        && value
            .as_bytes()
            .first()
            .is_some_and(|byte| byte.is_ascii_alphabetic())
}

fn arcadia_debug_pathname_is_safe(value: &str) -> bool {
    value.starts_with('/')
        && !value.starts_with("//")
        && !value.contains("://")
        && !value.contains('\n')
        && !value.contains('\r')
        && !value.split('/').any(|part| part == "..")
        && value.len() <= 256
}

fn arcadia_debug_enum(value: Option<&str>, allowed: &[&str]) -> Option<String> {
    value
        .filter(|item| allowed.contains(item))
        .map(arcadia_debug_trim_text)
}

fn arcadia_debug_key_is_sensitive(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    ["token", "pin", "password", "headers", "localstorage", "dom"]
        .iter()
        .any(|needle| key.contains(needle))
}

fn arcadia_debug_trim_text(value: &str) -> String {
    value.chars().take(ARCADIA_DEBUG_MAX_TEXT).collect()
}

fn sanitize_arcadia_debug_value(
    value: &serde_json::Value,
    depth: usize,
) -> Option<serde_json::Value> {
    if depth > ARCADIA_DEBUG_MAX_DEPTH {
        return None;
    }
    match value {
        serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::Number(_) => {
            Some(value.clone())
        }
        serde_json::Value::String(text) => {
            Some(serde_json::Value::String(arcadia_debug_trim_text(text)))
        }
        serde_json::Value::Array(values) => Some(serde_json::Value::Array(
            values
                .iter()
                .take(ARCADIA_DEBUG_MAX_ARRAY_ITEMS)
                .filter_map(|value| sanitize_arcadia_debug_value(value, depth + 1))
                .collect(),
        )),
        serde_json::Value::Object(values) => Some(serde_json::Value::Object(
            values
                .iter()
                .filter(|(key, _)| !arcadia_debug_key_is_sensitive(key))
                .take(ARCADIA_DEBUG_MAX_ITEMS)
                .filter_map(|(key, value)| {
                    sanitize_arcadia_debug_value(value, depth + 1).map(|value| (key.clone(), value))
                })
                .collect(),
        )),
    }
}

fn arcadia_debug_reflection(body: &serde_json::Value) -> Result<serde_json::Value, &'static str> {
    let kind = body
        .get("kind")
        .and_then(|value| value.as_str())
        .ok_or("debug-kind-required")?;
    if !arcadia_debug_kind_is_safe(kind) {
        return Err("debug-kind-invalid");
    }
    let string = |name: &str| {
        body.get(name)
            .and_then(|value| value.as_str())
            .filter(|value| !value.chars().any(char::is_control))
            .map(arcadia_debug_trim_text)
    };
    let identifier = |name: &str| {
        body.get(name)
            .and_then(|value| value.as_str())
            .filter(|value| arcadia_debug_identifier_is_safe(value))
            .map(arcadia_debug_trim_text)
    };
    let pathname = body
        .get("pathname")
        .and_then(|value| value.as_str())
        .filter(|value| arcadia_debug_pathname_is_safe(value))
        .map(arcadia_debug_trim_text);
    let number = |name: &str| body.get(name).and_then(|value| value.as_u64());
    let attributes = body
        .get("attributes")
        .or_else(|| body.get("payload"))
        .and_then(|value| value.as_object())
        .cloned()
        .unwrap_or_default();
    Ok(serde_json::json!({
        "organ": "arcadia", "kind": kind,
        "event": arcadia_debug_enum(body.get("event").and_then(|value| value.as_str()), &["begin", "mark", "settled", "response", "fault", "action", "presenter", "runtime", "currentness", "stream", "view-change", "coalesced", "bounded", "boot", "ready", "status"]),
        "outcome": arcadia_debug_enum(body.get("outcome").and_then(|value| value.as_str()), &["begin", "observed", "ok", "refused", "fault", "opened", "closed", "confirmed", "current", "changed", "stale", "recovered", "ready", "boot", "degraded", "dropped", "settled"]), "observed_at": string("observed_at"),
        "correlation_id": identifier("correlation_id"), "span_id": identifier("span_id"), "parent_span_id": identifier("parent_span_id"),
        "sequence": number("sequence"), "duration_ms": number("duration_ms"), "method": arcadia_debug_enum(body.get("method").and_then(|value| value.as_str()), &["GET", "POST", "PUT", "PATCH", "DELETE"]),
        "pathname": pathname, "status": number("status"), "surface_id": identifier("surface_id"),
        "surface_class": identifier("surface_class"), "action_id": identifier("action_id"), "currentness": arcadia_debug_enum(body.get("currentness").and_then(|value| value.as_str()), &["current", "changed", "stale", "recovered"]),
        "level": body.get("level").and_then(|value| value.as_str()).filter(|level| ["debug", "info", "warn"].contains(level)).unwrap_or("debug"),
        "message": body.get("message").and_then(|value| value.as_str()).map(arcadia_debug_trim_text).unwrap_or_else(|| format!("Arcadia debug event: {kind}")),
        "attributes_redacted": sanitize_arcadia_debug_value(&serde_json::Value::Object(attributes), 0).unwrap_or(serde_json::Value::Null),
    }))
}

fn forward_arcadia_debug_reflection(reflection: &serde_json::Value) -> bool {
    let Ok(rendered) = serde_json::to_string(reflection) else {
        return false;
    };
    caduceus_post_json_with_timeout("/api/v1/hyalos/reflect", &rendered, "2").is_ok()
}

async fn arcadia_debug_emit_route(Json(body): Json<serde_json::Value>) -> impl IntoResponse {
    let Ok(reflection) = arcadia_debug_reflection(&body) else {
        return StatusCode::BAD_REQUEST;
    };
    if forward_arcadia_debug_reflection(&reflection) {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
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

async fn caduceus_vault_status_proxy_route() -> impl IntoResponse {
    caduceus_json_proxy("/api/v1/vault/status").await
}
async fn caduceus_vault_unlock_proxy_route(
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    caduceus_json_post_proxy("/api/v1/vault/unlock", body)
}
async fn caduceus_vault_auto_decrypt_proxy_route(
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    caduceus_json_post_proxy("/api/v1/vault/auto-decrypt", body)
}
fn caduceus_json_post_proxy(
    path: &'static str,
    body: serde_json::Value,
) -> axum::response::Response {
    let rendered = serde_json::to_string(&body).unwrap_or_else(|_| "{}".to_string());
    match caduceus_post_json(path, &rendered) {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(signal) => caduceus_proxy_error(path, signal),
    }
}
async fn caduceus_update_status_proxy_route() -> impl IntoResponse {
    caduceus_json_proxy("/api/v1/update/status").await
}

async fn caduceus_cert_status_proxy_route() -> impl IntoResponse {
    caduceus_json_proxy("/api/v1/cert/status").await
}

async fn caduceus_cert_trust_install_proxy_route(
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let mut payload = body.as_object().cloned().unwrap_or_default();
    payload.entry("bundle".to_string()).or_insert_with(|| {
        serde_json::json!("/var/lib/caduceus/certs/bundles/homeserver-house-ca-linux.crt")
    });
    let rendered =
        serde_json::to_string(&serde_json::Value::Object(payload)).unwrap_or_else(|_| {
            "{\"bundle\":\"/var/lib/caduceus/certs/bundles/homeserver-house-ca-linux.crt\"}"
                .to_string()
        });
    match caduceus_post_json("/api/v1/cert/trust-install", &rendered) {
        Ok(value) => {
            let ok = value.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
            let status = if ok {
                StatusCode::OK
            } else {
                StatusCode::BAD_GATEWAY
            };
            (status, Json(value)).into_response()
        }
        Err(signal) => caduceus_proxy_error("/api/v1/cert/trust-install", signal),
    }
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
    let rendered =
        serde_json::to_string(&payload).unwrap_or_else(|_| "{\"state\":\"on\"}".to_string());
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
    let rendered = serde_json::to_string(&body)
        .unwrap_or_else(|_| "{\"module_id\":\"\",\"enabled\":false}".to_string());
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
            if action == "sync-games" {
                tracing::info!(
                    kind = "sync-run", ok, result = if ok { "success" } else { "error" },
                    duration_ms = ?value.get("durationMs").or_else(|| value.get("duration_ms")).unwrap_or(&serde_json::Value::Null),
                    counts = ?value.get("counts").unwrap_or(&serde_json::Value::Null),
                    outcomes = ?value.get("outcomes").unwrap_or(&serde_json::Value::Null),
                    receipt_ref = ?value.get("receiptRef").or_else(|| value.get("receipt_ref")).unwrap_or(&serde_json::Value::Null),
                    "sync games outcome"
                );
            }
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
            console_action_error(StatusCode::BAD_GATEWAY, action, "caduceus-http", signal)
        }
    }
}

#[cfg(test)]
mod arcadia_debug_tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::{Shutdown, TcpListener},
        sync::{Arc, Mutex, OnceLock},
        thread,
    };

    static CADUCEUS_ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    #[test]
    fn debug_route_forwards_only_bounded_redacted_reflections() {
        let _guard = CADUCEUS_ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").expect("mock caduceus bind");
        let port = listener.local_addr().expect("mock caduceus address").port();
        let captured = Arc::new(Mutex::new(String::new()));
        let captured_thread = captured.clone();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("Arcadia debug request");
            let mut buf = [0u8; 8192];
            let read = stream.read(&mut buf).expect("debug request bytes");
            *captured_thread.lock().unwrap() = String::from_utf8_lossy(&buf[..read]).to_string();
            let response = r#"{"ok":true}"#;
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}", response.len()).unwrap();
            let _ = stream.shutdown(Shutdown::Write);
        });
        let reflection = arcadia_debug_reflection(&serde_json::json!({
            "kind": "runtime",
            "event": "ready", "outcome": "ok", "span_id": "span-root", "parent_span_id": "span-parent",
            "sequence": 7, "duration_ms": 12, "pathname": "/api/root", "status": 200,
            "surface_id": "modal:wifi", "action_id": "wifi-scan", "currentness": "current",
            "message": "x".repeat(600),
            "payload": {"token": "secret", "pin": "1234", "ok": true, "items": (0..40).collect::<Vec<_>>()}
        })).expect("safe debug reflection");
        std::env::set_var("CADUCEUS_HTTP_BASE", format!("http://127.0.0.1:{port}"));
        assert!(forward_arcadia_debug_reflection(&reflection));
        handle.join().unwrap();
        std::env::remove_var("CADUCEUS_HTTP_BASE");

        let request = captured.lock().unwrap().clone();
        assert!(
            request.starts_with("POST /api/v1/hyalos/reflect HTTP/1.1"),
            "{request}"
        );
        let body: serde_json::Value =
            serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap_or("{}"))
                .expect("reflection JSON");
        assert_eq!(body["organ"], "arcadia");
        assert_eq!(body["kind"], "runtime");
        assert_eq!(body["event"], "ready");
        assert_eq!(body["outcome"], "ok");
        assert_eq!(body["span_id"], "span-root");
        assert_eq!(body["parent_span_id"], "span-parent");
        assert_eq!(body["sequence"], 7);
        assert_eq!(body["duration_ms"], 12);
        assert_eq!(body["pathname"], "/api/root");
        assert_eq!(body["status"], 200);
        assert_eq!(body["surface_id"], "modal:wifi");
        assert_eq!(body["action_id"], "wifi-scan");
        assert_eq!(body["currentness"], "current");
        assert_eq!(body["attributes_redacted"]["ok"], true);
        assert!(body["attributes_redacted"].get("token").is_none());
        assert!(body["attributes_redacted"].get("pin").is_none());
        assert_eq!(
            body["attributes_redacted"]["items"]
                .as_array()
                .unwrap()
                .len(),
            ARCADIA_DEBUG_MAX_ARRAY_ITEMS
        );
        assert_eq!(
            body["message"].as_str().unwrap().chars().count(),
            ARCADIA_DEBUG_MAX_TEXT
        );
    }

    #[test]
    fn debug_route_rejects_unsafe_kind_and_removes_unconditional_control_emission() {
        assert!(arcadia_debug_reflection(&serde_json::json!({"kind": "Control Action"})).is_err());
        let source = include_str!("routes_caduceus.rs");
        let main = include_str!("../main.rs");
        assert!(main.contains("/api/debug/emit"));
        assert!(
            source.contains("DefaultBodyLimit")
                || main.contains("DefaultBodyLimit::max(16 * 1024)")
        );
        let legacy_emitter = ["caduceus_hyalos_", "reflect_action"].concat();
        assert!(!source.contains(&legacy_emitter));
        let channel_path = ["channel", ".jsonl"].concat();
        assert!(!source.contains(&channel_path));
    }
}
