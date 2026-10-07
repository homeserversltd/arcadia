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
    CaduceusAccessClient::default().post_json_with_timeout(
        "/api/v1/log/reflect",
        serde_json::from_str(&rendered).unwrap_or_default(),
        Duration::from_secs(2)
    ).is_ok()
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
    match CaduceusAccessClient::default().get_json(path) {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(signal) => caduceus_proxy_error(path, signal),
    }
}

async fn caduceus_model_lanes_pulse_proxy_route() -> impl IntoResponse {
    match CaduceusAccessClient::default().post_json_with_timeout(
        "/api/v1/appliance/model-lanes/pulse",
        serde_json::json!({}),
        Duration::from_secs(300)
    ) {
        Ok(value) => {
            let ok = value.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
            let status = if ok {
                StatusCode::OK
            } else {
                StatusCode::BAD_GATEWAY
            };
            (status, Json(value)).into_response()
        }
        Err(signal) => caduceus_proxy_error("/api/v1/appliance/model-lanes/pulse", signal),
    }
}

async fn caduceus_health_proxy_route() -> impl IntoResponse {
    caduceus_json_proxy("/health").await
}

async fn caduceus_identity_proxy_route() -> impl IntoResponse {
    caduceus_json_proxy("/api/v1/appliance/report").await
}

async fn caduceus_profile_proxy_route() -> impl IntoResponse {
    caduceus_json_proxy("/api/v1/appliance/report").await
}

async fn caduceus_health_api_proxy_route() -> impl IntoResponse {
    caduceus_json_proxy("/health").await
}

async fn caduceus_vault_status_proxy_route() -> impl IntoResponse {
    let fallback = || {
        (
            StatusCode::OK,
            Json(serde_json::json!({
                "present": false,
                "mounted": false,
                "auto_decrypt_enabled": false,
                "unlock_required": false,
            })),
        )
            .into_response()
    };
    match CaduceusAccessClient::default().get_json("/api/v1/storage/vault/status") {
        Ok(value) => {
            let present = value.get("present").and_then(|item| item.as_bool()).unwrap_or(false);
            let Some(mounted) = value.get("mounted").and_then(|item| item.as_bool()) else {
                return fallback();
            };
            let Some(auto_decrypt_enabled) = value
                .get("auto_decrypt_enabled")
                .and_then(|item| item.as_bool())
            else {
                return fallback();
            };
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "present": present,
                    "mounted": mounted,
                    "auto_decrypt_enabled": auto_decrypt_enabled,
                    "unlock_required": present && !mounted && !auto_decrypt_enabled,
                })),
            )
                .into_response()
        }
        Err(_) => fallback(),
    }
}
async fn caduceus_vault_unlock_proxy_route(
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Response {
    caduceus_vault_unlock_route(headers, Json(body)).await
}
async fn caduceus_vault_auto_decrypt_proxy_route(
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Response {
    caduceus_vault_auto_decrypt_route(headers, Json(body)).await
}
fn caduceus_json_post_proxy(
    path: &'static str,
    body: serde_json::Value,
) -> axum::response::Response {
    let rendered = serde_json::to_string(&body).unwrap_or_else(|_| "{}".to_string());
    match CaduceusAccessClient::default().post_json_with_timeout(
        path,
        serde_json::from_str(&rendered).unwrap_or_default(),
        Duration::from_secs(300)
    ) {
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

async fn caduceus_cert_trust_fetch_proxy_route(
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let body = serde_json::json!({
        "server": body.get("server").and_then(|value| value.as_str()).unwrap_or(""),
        "platform": body.get("platform").and_then(|value| value.as_str()).unwrap_or("linux"),
    });
    let rendered = serde_json::to_string(&body).unwrap_or_else(|_| "{}".to_string());
    match CaduceusAccessClient::default().post_json_with_timeout(
        "/api/v1/cert/trust-fetch",
        serde_json::from_str(&rendered).unwrap_or_default(),
        Duration::from_secs(300)
    ) {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(signal) => caduceus_proxy_error("/api/v1/cert/trust-fetch", signal),
    }
}


async fn caduceus_update_now_proxy_route() -> impl IntoResponse {
    match CaduceusAccessClient::default().post_json_with_timeout(
        "/api/v1/update/now",
        serde_json::json!({}),
        Duration::from_secs(300)
    ) {
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
    match CaduceusAccessClient::default().post_json_with_timeout(
        "/api/v1/update/check",
        serde_json::json!({}),
        Duration::from_secs(300)
    ) {
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
    caduceus_json_proxy("/api/v1/update/status").await
}

async fn caduceus_sync_now_proxy_route() -> impl IntoResponse {
    match CaduceusAccessClient::default().post_json_with_timeout(
        "/api/v1/update/now",
        serde_json::json!({}),
        Duration::from_secs(300)
    ) {
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

async fn caduceus_receipts_latest_proxy_route() -> impl IntoResponse {
    caduceus_json_proxy("/api/v1/log/receipts").await
}

async fn caduceus_receipts_ledger_proxy_route(
    Query(query): Query<HarmoniaLedgerQuery>,
) -> impl IntoResponse {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(10).clamp(1, 25);
    let path = format!("/api/v1/log/receipts?page={page}&per_page={per_page}");
    match CaduceusAccessClient::default().get_json(&path) {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(signal) => caduceus_proxy_error(&path, signal),
    }
}

async fn caduceus_update_service_status_proxy_route() -> impl IntoResponse {
    caduceus_json_proxy("/api/v1/update/status").await
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
    match CaduceusAccessClient::default().post_json_with_timeout(
        "/api/v1/update/now",
        serde_json::from_str(&rendered).unwrap_or_default(),
        Duration::from_secs(300)
    ) {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(signal) => caduceus_proxy_error("/api/v1/update/now", signal),
    }
}

async fn caduceus_gui_update_now_proxy_route() -> impl IntoResponse {
    match CaduceusAccessClient::default().post_json_with_timeout(
        "/api/v1/update/now",
        serde_json::json!({}),
        Duration::from_secs(300)
    ) {
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

async fn caduceus_local_ai_runtime_status_proxy_route() -> impl IntoResponse {
    caduceus_json_proxy("/api/v1/local-ai/runtime/status").await
}

async fn caduceus_local_ai_runtime_check_proxy_route() -> impl IntoResponse {
    match CaduceusAccessClient::default().post_json_with_timeout(
        "/api/v1/doors",
        serde_json::json!({}),
        Duration::from_secs(300)
    ) {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(signal) => caduceus_proxy_error("/api/v1/doors", signal),
    }
}

async fn caduceus_local_ai_runtime_update_proxy_route() -> impl IntoResponse {
    match CaduceusAccessClient::default().post_json_with_timeout(
        "/api/v1/doors",
        serde_json::json!({}),
        Duration::from_secs(300)
    ) {
        Ok(value) => {
            let ok = value.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
            let status = if ok {
                StatusCode::OK
            } else {
                StatusCode::BAD_GATEWAY
            };
            (status, Json(value)).into_response()
        }
        Err(signal) => caduceus_proxy_error("/api/v1/doors", signal),
    }
}

async fn caduceus_profile_module_toggle_proxy_route(
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let module_id = body
        .get("module_id")
        .or_else(|| body.get("moduleId"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .unwrap_or("");
    if module_id.is_empty() || module_id.len() > 96 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "ok": false,
                "first_missing_signal": "module-id-invalid",
            })),
        )
            .into_response();
    }
    let Some(enabled) = body.get("enabled").and_then(serde_json::Value::as_bool) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "ok": false,
                "first_missing_signal": "module-toggle-enabled-invalid",
            })),
        )
            .into_response();
    };
    let path = format!(
        "/api/v1/update/modules/{}",
        caduceus_encode_path_segment(module_id),
    );
    match CaduceusAccessClient::default().post_raw_json_with_timeout(
        &path,
        serde_json::json!({ "enabled": enabled }),
        Duration::from_secs(300),
    ) {
        Ok((upstream_status, value)) if !(200..300).contains(&upstream_status) => {
            let signal = value
                .get("firstMissingSignal")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("caduceus-http-upstream-refused");
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
        Ok((_upstream_status, value)) => {
            let ok = value
                .get("ok")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            let status = if ok {
                StatusCode::OK
            } else {
                StatusCode::BAD_REQUEST
            };
            // Forward the complete receipt so id, enabled, and unknown fields survive.
            (status, Json(value)).into_response()
        }
        Err(signal) => caduceus_proxy_error(&path, signal),
    }
}

async fn caduceus_profile_module_update_proxy_route(
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let module_id = body
        .get("module_id")
        .or_else(|| body.get("moduleId"))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .unwrap_or("");
    if !valid_harmonia_module_id(module_id) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "ok": false,
                "action": "update-module",
                "module_id": module_id,
                "first_missing_signal": "module-id-invalid",
                "message": "Module id must be lowercase letters, numbers, and hyphens only.",
            })),
        )
            .into_response();
    }
    if !harmonia_independent_update_allowed(module_id) {
        return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "ok": false,
                "action": "update-module",
                "module_id": module_id,
                "first_missing_signal": "module-independent-update-unavailable",
                "message": "Harmonia membership does not permit an independent module update.",
            })),
        )
            .into_response();
    }
    match CaduceusAccessClient::default().post_json_with_target(
        "/api/v1/update/module",
        serde_json::json!({ "module_id": module_id }),
        Duration::from_secs(300),
        serde_json::json!({ "apply": true }),
        serde_json::json!({ "module": module_id }),
    ) {
        Ok((status, value)) => {
            let status = StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY);
            (status, Json(value)).into_response()
        }
        Err(signal) => caduceus_proxy_error("/api/v1/update/module", signal),
    }
}

fn run_caduceus_http_mutation(
    action: &'static str,
    path: &'static str,
    success_message: &str,
    failure_message: &str,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    match CaduceusAccessClient::default().post_json_with_timeout(
        path,
        serde_json::json!({}),
        Duration::from_secs(300)
    ) {
        Ok(value) => {
            let ok = value.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
            let stdout = value
                .get("body")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let signal = value
                .get("first_missing_signal")
                .or_else(|| value.get("firstMissingSignal"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if action == "sync-games" {
                let duration_ms = serde_json::to_string(
                    value.get("durationMs").or_else(|| value.get("duration_ms")).unwrap_or(&serde_json::Value::Null),
                )
                .unwrap_or_else(|_| "null".to_string());
                let counts = serde_json::to_string(value.get("counts").unwrap_or(&serde_json::Value::Null))
                    .unwrap_or_else(|_| "null".to_string());
                let outcomes = serde_json::to_string(value.get("outcomes").unwrap_or(&serde_json::Value::Null))
                    .unwrap_or_else(|_| "null".to_string());
                tracing::info!(
                    kind = "sync-run", ok, result = if ok { "success" } else { "error" },
                    duration_ms = duration_ms.as_str(), counts = counts.as_str(), outcomes = outcomes.as_str(),
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
        os::unix::net::{UnixListener, UnixStream},
        sync::{Arc, Mutex, OnceLock},
        thread,
    };

    static CADUCEUS_ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn read_http_request(stream: &mut UnixStream) -> String {
        const MAX_REQUEST_BYTES: usize = 16 * 1024;
        let mut request = Vec::new();
        loop {
            let mut byte = [0_u8; 1];
            stream.read_exact(&mut byte).expect("HTTP request header bytes");
            request.push(byte[0]);
            if request.ends_with(b"\r\n\r\n") {
                break;
            }
            assert!(request.len() < MAX_REQUEST_BYTES, "HTTP request headers exceed bound");
        }

        let header_text = String::from_utf8_lossy(&request);
        let content_length = header_text
            .split("\r\n")
            .filter_map(|line| line.split_once(':'))
            .find_map(|(name, value)| {
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().expect("valid Content-Length"))
            })
            .unwrap_or(0);
        assert!(
            request.len().checked_add(content_length).is_some_and(|length| length <= MAX_REQUEST_BYTES),
            "HTTP request exceeds bound"
        );
        let body_start = request.len();
        request.resize(body_start + content_length, 0);
        stream
            .read_exact(&mut request[body_start..])
            .expect("HTTP request body bytes");
        String::from_utf8_lossy(&request).into_owned()
    }

    #[test]
    fn debug_route_forwards_only_bounded_redacted_reflections() {
        let _guard = CADUCEUS_ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let socket = std::env::temp_dir().join(format!("arcadia-caduceus-{}.sock", uuid::Uuid::new_v4()));
        let listener = UnixListener::bind(&socket).expect("mock caduceus bind");
        let captured = Arc::new(Mutex::new(String::new()));
        let captured_thread = captured.clone();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("Arcadia debug request");
            *captured_thread.lock().unwrap() = read_http_request(&mut stream);
            let response = r#"{"ok":true}"#;
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}", response.len()).unwrap();
            let _ = stream.shutdown(std::net::Shutdown::Write);
        });
        let reflection = arcadia_debug_reflection(&serde_json::json!({
            "kind": "runtime",
            "event": "ready", "outcome": "ok", "span_id": "span-root", "parent_span_id": "span-parent",
            "sequence": 7, "duration_ms": 12, "pathname": "/api/root", "status": 200,
            "surface_id": "modal:wifi", "action_id": "wifi-scan", "currentness": "current",
            "message": "x".repeat(600),
            "payload": {"token": "secret", "pin": "1234", "ok": true, "items": (0..40).collect::<Vec<_>>()}
        })).expect("safe debug reflection");
        std::env::set_var("CADUCEUS_STAFF_SOCKET", &socket);
        assert!(forward_arcadia_debug_reflection(&reflection));
        handle.join().unwrap();
        std::env::remove_var("CADUCEUS_STAFF_SOCKET");
        let _ = std::fs::remove_file(&socket);

        let request = captured.lock().unwrap().clone();
        assert!(
            request.starts_with("POST /api/v1/log/reflect HTTP/1.1"),
            "{request}"
        );
        let body: serde_json::Value =
            serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap_or("{}"))
                .expect("reflection JSON");
        assert_eq!(body["schema"], "caduceus.staff.v1");
        assert_eq!(body["transition"], "log.reflect");
        assert_eq!(body["version"], 1);
        assert!(body["timestamp"].as_u64().is_some());
        assert!(body["target"].is_null());
        assert!(body["flags"]["exousia"].is_null());
        let body = &body["payload"];
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
    fn staff_socket_reads_and_writes_without_tcp_fallback() {
        let _guard = CADUCEUS_ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let socket = std::env::temp_dir().join(format!("arcadia-caduceus-{}.sock", uuid::Uuid::new_v4()));
        let listener = UnixListener::bind(&socket).expect("mock caduceus bind");
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("staff request");
            let request = read_http_request(&mut stream);
            assert!(request.starts_with("POST /api/v1/test HTTP/1.1"));
            assert!(request.contains("Host: localhost\r\n"));
            let body = request.split("\r\n\r\n").nth(1).unwrap_or("");
            let value: serde_json::Value = serde_json::from_str(body).expect("request JSON");
            assert_eq!(value["payload"]["answer"], 42);
            assert_eq!(value["schema"], "caduceus.staff.v1");
            assert_eq!(value["transition"], "test");
            assert_eq!(value["version"], 1);
            assert!(value["target"].is_null());
            assert!(value["flags"]["exousia"].is_null());
            let response = r#"{"ok":true}"#;
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{response}", response.len()).unwrap();
        });
        std::env::set_var("CADUCEUS_STAFF_SOCKET", &socket);
        let value = CaduceusAccessClient::default().post_json("/api/v1/test", serde_json::json!({"answer": 42})).expect("staff response");
        handle.join().unwrap();
        std::env::remove_var("CADUCEUS_STAFF_SOCKET");
        let _ = std::fs::remove_file(&socket);
        assert_eq!(value["ok"], true);
    }

    #[test]
    fn staff_socket_get_reads_json_over_unix_listener() {
        let _guard = CADUCEUS_ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let socket = std::env::temp_dir().join(format!("arcadia-caduceus-read-{}.sock", uuid::Uuid::new_v4()));
        let listener = UnixListener::bind(&socket).expect("mock caduceus bind");
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("staff GET");
            let request = read_http_request(&mut stream);
            assert!(request.starts_with("GET /api/v1/health HTTP/1.1"));
            let response = r#"{"healthy":true}"#;
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{response}", response.len()).unwrap();
        });
        std::env::set_var("CADUCEUS_STAFF_SOCKET", &socket);
        let value = CaduceusAccessClient::default().get_json("/api/v1/health").expect("staff GET response");
        handle.join().unwrap();
        std::env::remove_var("CADUCEUS_STAFF_SOCKET");
        let _ = std::fs::remove_file(&socket);
        assert_eq!(value["healthy"], true);
    }

    #[test]
    fn observation_response_body_cap_is_larger_than_command_receipts() {
        let _guard = CADUCEUS_ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let socket = std::env::temp_dir().join(format!(
            "arcadia-caduceus-observation-cap-{}.sock",
            uuid::Uuid::new_v4()
        ));
        let listener = UnixListener::bind(&socket).expect("mock caduceus bind");
        let handle = thread::spawn(move || {
            let observation = serde_json::json!({"body": "x".repeat(17 * 1024)});
            let observation = serde_json::to_vec(&observation).unwrap();
            let oversized_length = CADUCEUS_MAX_OBSERVATION_RESPONSE_BODY + 1;
            for (body, content_length) in [
                (Some(observation), None),
                (None, Some(oversized_length)),
            ] {
                let (mut stream, _) = listener.accept().expect("observation GET");
                let request = read_http_request(&mut stream);
                assert!(request.starts_with("GET /api/v1/appliance/stats/history HTTP/1.1"));
                let length = content_length.unwrap_or_else(|| body.as_ref().unwrap().len());
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n"
                )
                .unwrap();
                if let Some(body) = body {
                    stream.write_all(&body).unwrap();
                }
            }
        });
        std::env::set_var("CADUCEUS_STAFF_SOCKET", &socket);
        let client = CaduceusAccessClient::default();
        let observation = client
            .get_json("/api/v1/appliance/stats/history")
            .expect("large observation response");
        assert_eq!(observation["body"].as_str().unwrap().len(), 17 * 1024);
        let oversized = client.get_json("/api/v1/appliance/stats/history");
        handle.join().unwrap();
        std::env::remove_var("CADUCEUS_STAFF_SOCKET");
        let _ = std::fs::remove_file(&socket);
        assert_eq!(oversized, Err("caduceus-http-empty-response"));
    }

    #[test]
    fn explicit_short_timeout_is_honored_by_staff_client() {
        let _guard = CADUCEUS_ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let socket = std::env::temp_dir().join(format!("arcadia-caduceus-timeout-{}.sock", uuid::Uuid::new_v4()));
        let listener = UnixListener::bind(&socket).expect("mock caduceus bind");
        let handle = thread::spawn(move || {
            let (_stream, _) = listener.accept().expect("staff request");
            thread::sleep(Duration::from_millis(150));
        });
        std::env::set_var("CADUCEUS_STAFF_SOCKET", &socket);
        let started = std::time::Instant::now();
        let result = CaduceusAccessClient::default().post_json_with_timeout(
            "/api/v1/test",
            serde_json::json!({}),
            Duration::from_millis(20)
        );
        let elapsed = started.elapsed();
        std::env::remove_var("CADUCEUS_STAFF_SOCKET");
        handle.join().unwrap();
        let _ = std::fs::remove_file(&socket);
        assert!(result.is_err());
        assert!(elapsed < Duration::from_millis(120));
    }

    #[test]
    fn retired_caduceus_transport_names_are_absent_from_production() {
        let retired = [
            concat!("caduceus_", "fetch_json"),
            concat!("caduceus_", "post_flat_json"),
            concat!("caduceus_", "post_json_with_timeout"),
        ];
        let production_sources = [
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/bands/caduceus_access.rs")),
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/bands/routes_caduceus.rs")),
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/bands/console_system_actions.rs")),
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/bands/routes_ai_models.rs")),
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/main.rs")),
        ];
        for source in production_sources {
            for name in retired {
                assert!(!source.contains(name), "retired helper remains: {name}");
            }
        }
        let curl_constructor = concat!("Command::new(", "\"curl\"", ")");
        assert!(!production_sources[0].contains(curl_constructor));
    }

    #[test]
    fn missing_staff_socket_fails_without_fallback() {
        let _guard = CADUCEUS_ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let socket = std::env::temp_dir().join(format!("arcadia-caduceus-missing-{}.sock", uuid::Uuid::new_v4()));
        std::env::set_var("CADUCEUS_STAFF_SOCKET", &socket);
        let result = CaduceusAccessClient::default().get_json("/api/v1/health");
        std::env::remove_var("CADUCEUS_STAFF_SOCKET");
        assert_eq!(result, Err("caduceus-http-unreachable"));
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


    #[test]
    fn attendance_open_sends_plain_json_and_accepts_opaque_proof() {
        let _guard = CADUCEUS_ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let socket = std::env::temp_dir().join(format!("aca-open-{}.sock", uuid::Uuid::new_v4()));
        let listener = UnixListener::bind(&socket).expect("mock caduceus bind");
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("attendance open request");
            let request = read_http_request(&mut stream);
            assert!(request.starts_with("POST /api/v1/exousia/open HTTP/1.1"));
            let body = request.split("\r\n\r\n").nth(1).unwrap_or("");
            let value: serde_json::Value = serde_json::from_str(body).expect("attendance open JSON");
            assert_eq!(value["pin"], "2468");
            assert_eq!(value["documentId"], "doc-open");
            assert_eq!(value["documentIncarnation"], "doc-open");
            for envelope_field in ["schema", "transition", "version", "timestamp", "target", "flags", "payload"] {
                assert!(value.get(envelope_field).is_none(), "unexpected staff field: {envelope_field}");
            }
            let response = r#"{"ok":true,"attendance":"attendance-proof"}"#;
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{response}", response.len()).unwrap();
        });
        std::env::set_var("CADUCEUS_STAFF_SOCKET", &socket);
        let call = CaduceusAccessClient::default().attendance_open("2468", "doc-open");
        handle.join().unwrap();
        std::env::remove_var("CADUCEUS_STAFF_SOCKET");
        let _ = std::fs::remove_file(&socket);
        assert!(call.ok);
        assert_eq!(call.status, 200);
        assert_eq!(call.code, "none");
        assert_eq!(call.proof.as_ref().map(AttendanceProof::expose), Some("attendance-proof"));
    }

    #[test]
    fn attendance_validate_sends_plain_json_and_accepts_success() {
        let _guard = CADUCEUS_ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let socket = std::env::temp_dir().join(format!("aca-valid-{}.sock", uuid::Uuid::new_v4()));
        let listener = UnixListener::bind(&socket).expect("mock caduceus bind");
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("attendance validate request");
            let request = read_http_request(&mut stream);
            assert!(request.starts_with("POST /api/v1/exousia/validate HTTP/1.1"));
            let body = request.split("\r\n\r\n").nth(1).unwrap_or("");
            let value: serde_json::Value = serde_json::from_str(body).expect("attendance validate JSON");
            assert_eq!(value["attendance"], "attendance-proof");
            assert_eq!(value["documentId"], "doc-current");
            assert_eq!(value["documentIncarnation"], "doc-current");
            for envelope_field in ["schema", "transition", "version", "timestamp", "target", "flags", "payload"] {
                assert!(value.get(envelope_field).is_none(), "unexpected staff field: {envelope_field}");
            }
            let response = r#"{"ok":true}"#;
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{response}", response.len()).unwrap();

            let (mut stream, _) = listener.accept().expect("attendance invalidate request");
            let request = read_http_request(&mut stream);
            assert!(request.starts_with("POST /api/v1/exousia/invalidate HTTP/1.1"));
            let body = request.split("\r\n\r\n").nth(1).unwrap_or("");
            let value: serde_json::Value = serde_json::from_str(body).expect("attendance invalidate JSON");
            assert_eq!(value["attendance"], "attendance-proof");
            assert_eq!(value["documentId"], "doc-current");
            assert_eq!(value["documentIncarnation"], "doc-current");
            for envelope_field in ["schema", "transition", "version", "timestamp", "target", "flags", "payload"] {
                assert!(value.get(envelope_field).is_none(), "unexpected staff field: {envelope_field}");
            }
            let response = r#"{"ok":false,"code":"caduceus-attendance-refused"}"#;
            write!(stream, "HTTP/1.1 401 Unauthorized\r\nContent-Length: {}\r\n\r\n{response}", response.len()).unwrap();
        });
        std::env::set_var("CADUCEUS_STAFF_SOCKET", &socket);
        let attendance = AttendanceProof::parse("attendance-proof").unwrap();
        let call = CaduceusAccessClient::default().attendance_validate(&attendance, "doc-current");
        let invalidate = CaduceusAccessClient::default().attendance_invalidate(&attendance, "doc-current");
        handle.join().unwrap();
        std::env::remove_var("CADUCEUS_STAFF_SOCKET");
        let _ = std::fs::remove_file(&socket);
        assert!(call.ok);
        assert_eq!(call.status, 200);
        assert_eq!(call.code, "none");
        assert!(call.proof.is_none());
        assert!(!invalidate.ok);
        assert_eq!(invalidate.status, 401);
        assert_eq!(invalidate.code, "caduceus-attendance-refused");
        assert!(invalidate.proof.is_none());
    }

    #[test]
    fn stats_and_history_are_plain_bodyless_gets() {
        let _guard = CADUCEUS_ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let socket = std::env::temp_dir().join(format!("aca-stats-{}.sock", uuid::Uuid::new_v4()));
        let listener = UnixListener::bind(&socket).expect("mock caduceus bind");
        let handle = thread::spawn(move || {
            let (mut stats_stream, _) = listener.accept().expect("stats GET");
            let stats_request = read_http_request(&mut stats_stream);
            assert!(stats_request.starts_with("GET /api/v1/appliance/stats HTTP/1.1"));
            assert_eq!(stats_request.split("\r\n\r\n").nth(1).unwrap_or(""), "");
            let stats_response = r#"{"model_lanes":[]}"#;
            write!(stats_stream, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{stats_response}", stats_response.len()).unwrap();

            let (mut history_stream, _) = listener.accept().expect("history GET");
            let history_request = read_http_request(&mut history_stream);
            assert!(history_request.starts_with("GET /api/v1/appliance/stats/history HTTP/1.1"));
            assert_eq!(history_request.split("\r\n\r\n").nth(1).unwrap_or(""), "");
            let history_response = r#"{"history":[]}"#;
            write!(history_stream, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{history_response}", history_response.len()).unwrap();
        });
        std::env::set_var("CADUCEUS_STAFF_SOCKET", &socket);
        let client = CaduceusAccessClient::default();
        let stats = client.get_json("/api/v1/appliance/stats").expect("stats response");
        let history = client.get_json("/api/v1/appliance/stats/history").expect("history response");
        handle.join().unwrap();
        std::env::remove_var("CADUCEUS_STAFF_SOCKET");
        let _ = std::fs::remove_file(&socket);
        assert_eq!(stats["model_lanes"], serde_json::json!([]));
        assert_eq!(history["history"], serde_json::json!([]));
    }

    #[test]
    fn missing_staff_socket_maps_attendance_not_found_to_connect_refused() {
        let _guard = CADUCEUS_ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let socket = std::env::temp_dir().join(format!(
            "hermes-caduceus-missing-staff-{}-{}.sock",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock before UNIX_EPOCH")
                .as_nanos(),
        ));
        let socket = socket.to_string_lossy().into_owned();
        std::env::set_var("CADUCEUS_STAFF_SOCKET", &socket);
        let call = CaduceusAccessClient::default().attendance_open("1234", "fixture-document");
        std::env::remove_var("CADUCEUS_STAFF_SOCKET");

        assert!(!call.ok);
        assert_eq!(call.code, "caduceus-attendance-connect-refused");
    }
}
