// Caduceus-owned document attendance. Arcadia projects guest/admin only; it never
// verifies a PIN or treats an attendance proof as authority without Caduceus.
const CADUCEUS_ACCESS_TIMEOUT: Duration = Duration::from_secs(3);
const CADUCEUS_ACCESS_MAX_REQUEST: usize = 4 * 1024;
const ATTENDANCE_CACHE_TTL: Duration = Duration::from_secs(5);
struct AttendanceVerdict {
    ok: OnceLock<bool>,
    invalidated: std::sync::atomic::AtomicBool,
    expires_at: Mutex<Option<std::time::Instant>>,
}

static ATTENDANCE_VERDICTS: OnceLock<Mutex<HashMap<(String, String), (Arc<AttendanceVerdict>, Option<std::time::Instant>)>>> = OnceLock::new();

fn attendance_verdict_cache() -> &'static Mutex<HashMap<(String, String), (Arc<AttendanceVerdict>, Option<std::time::Instant>)>> {
    ATTENDANCE_VERDICTS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn attendance_cache_bust(document: &str, attendance: &AttendanceProof) {
    if let Ok(mut cache) = attendance_verdict_cache().lock() {
        if let Some((verdict, _)) = cache.remove(&(document.to_string(), attendance.expose().to_string())) {
            verdict.invalidated.store(true, Ordering::Release);
        }
    }
}

fn attendance_cached_validate(document: &str, attendance: &AttendanceProof) -> bool {
    let key = (document.to_string(), attendance.expose().to_string());
    let now = std::time::Instant::now();
    let verdict = if let Ok(mut cache) = attendance_verdict_cache().lock() {
        cache.retain(|_, (_, expires)| expires.is_none_or(|expires| expires > now));
        if let Some((verdict, _)) = cache.get(&key) { verdict.clone() }
        else {
            let verdict = Arc::new(AttendanceVerdict {
                ok: OnceLock::new(),
                invalidated: std::sync::atomic::AtomicBool::new(false),
                expires_at: Mutex::new(None),
            });
            cache.insert(key.clone(), (verdict.clone(), None));
            verdict
        }
    } else { return false; };
    let valid = *verdict.ok.get_or_init(|| CaduceusAccessClient::default().attendance_validate(attendance, document).ok)
        && !verdict.invalidated.load(Ordering::Acquire);
    let expires_at = if valid {
        verdict.expires_at.lock().ok().map(|mut expires| {
            *expires.get_or_insert_with(|| std::time::Instant::now() + ATTENDANCE_CACHE_TTL)
        })
    } else { None };
    if let Ok(mut cache) = attendance_verdict_cache().lock() {
        if cache.get(&key).is_some_and(|(cached, _)| Arc::ptr_eq(cached, &verdict)) {
            if let Some(expires_at) = expires_at.filter(|_| !verdict.invalidated.load(Ordering::Acquire)) {
                cache.insert(key, (verdict.clone(), Some(expires_at)));
            } else {
                cache.remove(&key);
            }
        }
    }
    valid && !verdict.invalidated.load(Ordering::Acquire)
}

#[derive(Clone, PartialEq, Eq)]
struct AttendanceProof(String);

impl std::fmt::Debug for AttendanceProof {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("AttendanceProof([redacted])")
    }
}

impl AttendanceProof {
    fn parse(raw: &str) -> Option<Self> {
        opaque_attendance(raw).then(|| Self(raw.to_string()))
    }

    fn expose(&self) -> &str {
        &self.0
    }
}

fn opaque_attendance(raw: &str) -> bool {
    !raw.is_empty()
        && raw.len() <= 1024
        && raw
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn safe_access_code(value: &str) -> String {
    if !value.is_empty()
        && value.len() <= 100
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        value.to_string()
    } else {
        "caduceus-attendance-refused".to_string()
    }
}

#[derive(Clone, Copy)]
enum AttendanceOperation {
    Open,
    Validate,
    Invalidate,
}

impl AttendanceOperation {
    fn path(self) -> &'static str {
        match self {
            Self::Open => "/api/v1/exousia/open",
            Self::Validate => "/api/v1/exousia/validate",
            Self::Invalidate => "/api/v1/exousia/invalidate",
        }
    }

    fn returns_proof(self) -> bool {
        matches!(self, Self::Open)
    }
}

struct AttendanceCall {
    ok: bool,
    status: u16,
    code: String,
    proof: Option<AttendanceProof>,
}

impl AttendanceCall {
    fn refused(status: u16, code: &str) -> Self {
        Self {
            ok: false,
            status,
            code: safe_access_code(code),
            proof: None,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ModelLane {
    pub alias: String,
    pub total_slots: Option<i64>,
    pub n_ctx_per_slot: Option<i64>,
    pub busy_slots: Option<i64>,
}

fn caduceus_staff_socket() -> String {
    env::var("CADUCEUS_STAFF_SOCKET").unwrap_or_else(|_| CADUCEUS_STAFF_SOCKET.to_string())
}

const CADUCEUS_MAX_RESPONSE_HEADERS: usize = 4096;
const CADUCEUS_MAX_COMMAND_RESPONSE_BODY: usize = 16 * 1024;
const CADUCEUS_MAX_OBSERVATION_RESPONSE_BODY: usize = 32 * 1024 * 1024;

#[derive(Debug)]
enum CaduceusTransportError {
    Connect(std::io::Error),
    Io(std::io::Error),
    Framing,
}

fn caduceus_raw_request(
    method: &str,
    path: &str,
    body: Option<&[u8]>,
    timeout: Duration,
    max_response_body: usize,
) -> Result<(u16, Vec<u8>), CaduceusTransportError> {
    use std::{io::Read, io::Write, os::unix::net::UnixStream};

    if path.bytes().any(|byte| matches!(byte, b'\r' | b'\n')) {
        return Err(CaduceusTransportError::Framing);
    }

    let mut stream = UnixStream::connect(caduceus_staff_socket())
        .map_err(CaduceusTransportError::Connect)?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(CaduceusTransportError::Io)?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(CaduceusTransportError::Io)?;

    let body = body.unwrap_or_default();
    if body.len() > CADUCEUS_ACCESS_MAX_REQUEST { return Err(CaduceusTransportError::Framing); }
    let request = format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Type: application/json\r\nAccept: application/json\r\nContent-Length: {}\r\n\r\n", body.len());
    stream
        .write_all(request.as_bytes())
        .and_then(|_| stream.write_all(body))
        .map_err(CaduceusTransportError::Io)?;

    let mut headers = Vec::new();
    loop {
        let mut byte = [0_u8; 1];
        stream
            .read_exact(&mut byte)
            .map_err(CaduceusTransportError::Io)?;
        headers.push(byte[0]);
        if headers.ends_with(b"\r\n\r\n") {
            break;
        }
        if headers.len() >= CADUCEUS_MAX_RESPONSE_HEADERS {
            return Err(CaduceusTransportError::Framing);
        }
    }

    let header_text = std::str::from_utf8(&headers).map_err(|_| CaduceusTransportError::Framing)?;
    let mut lines = header_text.split("\r\n");
    let status_line = lines.next().ok_or(CaduceusTransportError::Framing)?;
    let mut status_parts = status_line.split_whitespace();
    if status_parts.next() != Some("HTTP/1.1") {
        return Err(CaduceusTransportError::Framing);
    }
    let status = status_parts
        .next()
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or(CaduceusTransportError::Framing)?;
    let mut content_length = None;
    for line in lines {
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                content_length = Some(
                    value
                        .trim()
                        .parse::<usize>()
                        .map_err(|_| CaduceusTransportError::Framing)?,
                );
            }
        }
    }

    let length = content_length.ok_or(CaduceusTransportError::Framing)?;
    if length > max_response_body {
        return Err(CaduceusTransportError::Framing);
    }
    const RESPONSE_CHUNK_SIZE: usize = 8 * 1024;
    let mut response = Vec::new();
    let mut remaining = length;
    while remaining > 0 {
        let chunk_size = remaining.min(RESPONSE_CHUNK_SIZE);
        let mut chunk = [0_u8; RESPONSE_CHUNK_SIZE];
        stream
            .read_exact(&mut chunk[..chunk_size])
            .map_err(CaduceusTransportError::Io)?;
        response.extend_from_slice(&chunk[..chunk_size]);
        remaining -= chunk_size;
    }

    Ok((status, response))
}

fn caduceus_http_error(error: CaduceusTransportError) -> &'static str {
    match error {
        CaduceusTransportError::Connect(_) | CaduceusTransportError::Io(_) => {
            "caduceus-http-unreachable"
        }
        CaduceusTransportError::Framing => "caduceus-http-empty-response",
    }
}

const CADUCEUS_STAFF_SCHEMA: &str = "caduceus.staff.v1";

fn caduceus_transition(path: &str) -> String {
    let path = path.split('?').next().unwrap_or(path).trim_matches('/');
    path.strip_prefix("api/v1/").unwrap_or(path).replace('/', ".")
}

fn caduceus_staff_envelope(path: &str, payload: serde_json::Value, flags: serde_json::Value) -> serde_json::Value {
    caduceus_staff_envelope_with_target(path, payload, flags, serde_json::Value::Null)
}

fn caduceus_staff_envelope_with_target(
    path: &str,
    payload: serde_json::Value,
    flags: serde_json::Value,
    target: serde_json::Value,
) -> serde_json::Value {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|value| value.as_secs()).unwrap_or(0);
    serde_json::json!({"schema": CADUCEUS_STAFF_SCHEMA, "intent_id": format!("arcadia.staff.{}-{}", caduceus_transition(path), uuid::Uuid::new_v4()), "transition": caduceus_transition(path), "version": 1, "timestamp": timestamp, "target": target, "flags": flags, "payload": payload})
}

fn caduceus_json_response(status: u16, response: Vec<u8>) -> Result<serde_json::Value, &'static str> {
    if !(200..300).contains(&status) { return Err("caduceus-http-unreachable"); }
    if response.is_empty() { return Err("caduceus-http-empty-response"); }
    serde_json::from_slice(&response).map_err(|_| "caduceus-http-invalid-json")
}

fn caduceus_receipt_response(_status: u16, response: Vec<u8>) -> Result<serde_json::Value, &'static str> {
    if response.is_empty() { return Err("caduceus-http-empty-response"); }
    serde_json::from_slice(&response).map_err(|_| "caduceus-http-invalid-json")
}

fn caduceus_encode_path_segment(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

struct CaduceusAccessClient;

impl Default for CaduceusAccessClient {
    fn default() -> Self { Self }
}

impl CaduceusAccessClient {
    fn get_json(&self, path: &str) -> Result<serde_json::Value, &'static str> {
        let (status, response) = caduceus_raw_request(
            "GET",
            path,
            None,
            CADUCEUS_ACCESS_TIMEOUT,
            CADUCEUS_MAX_OBSERVATION_RESPONSE_BODY,
        ).map_err(caduceus_http_error)?;
        caduceus_json_response(status, response)
    }

    fn post_json(&self, path: &str, payload: serde_json::Value) -> Result<serde_json::Value, &'static str> {
        self.post_json_with_timeout_and_flags(path, payload, CADUCEUS_ACCESS_TIMEOUT, serde_json::Value::Null)
    }

    fn post_json_with_timeout(&self, path: &str, payload: serde_json::Value, timeout: Duration) -> Result<serde_json::Value, &'static str> {
        self.post_json_with_timeout_and_flags(path, payload, timeout, serde_json::Value::Null)
    }

    fn post_json_with_timeout_and_flags(&self, path: &str, payload: serde_json::Value, timeout: Duration, flags: serde_json::Value) -> Result<serde_json::Value, &'static str> {
        let encoded = serde_json::to_vec(&caduceus_staff_envelope(path, payload, flags)).map_err(|_| "caduceus-http-invalid-json")?;
        if encoded.len() > CADUCEUS_ACCESS_MAX_REQUEST { return Err("caduceus-http-request-too-large"); }
        let (status, response) = caduceus_raw_request(
            "POST",
            path,
            Some(&encoded),
            timeout,
            CADUCEUS_MAX_COMMAND_RESPONSE_BODY,
        ).map_err(caduceus_http_error)?;
        caduceus_json_response(status, response)
    }

    fn post_json_with_target(
        &self,
        path: &str,
        payload: serde_json::Value,
        timeout: Duration,
        flags: serde_json::Value,
        target: serde_json::Value,
    ) -> Result<(u16, serde_json::Value), &'static str> {
        let encoded = serde_json::to_vec(&caduceus_staff_envelope_with_target(path, payload, flags, target))
            .map_err(|_| "caduceus-http-invalid-json")?;
        if encoded.len() > CADUCEUS_ACCESS_MAX_REQUEST {
            return Err("caduceus-http-request-too-large");
        }
        let (status, response) = caduceus_raw_request(
            "POST",
            path,
            Some(&encoded),
            timeout,
            CADUCEUS_MAX_COMMAND_RESPONSE_BODY,
        )
        .map_err(caduceus_http_error)?;
        let value = caduceus_receipt_response(status, response)?;
        Ok((status, value))
    }

    fn post_raw_json_with_timeout(
        &self,
        path: &str,
        payload: serde_json::Value,
        timeout: Duration,
    ) -> Result<(u16, serde_json::Value), &'static str> {
        let encoded = serde_json::to_vec(&payload).map_err(|_| "caduceus-http-invalid-json")?;
        if encoded.len() > CADUCEUS_ACCESS_MAX_REQUEST {
            return Err("caduceus-http-request-too-large");
        }
        let (status, response) = caduceus_raw_request(
            "POST",
            path,
            Some(&encoded),
            timeout,
            CADUCEUS_MAX_COMMAND_RESPONSE_BODY,
        )
        .map_err(caduceus_http_error)?;
        let value = caduceus_receipt_response(status, response)?;
        Ok((status, value))
    }

    fn model_lanes(&self) -> Result<Vec<ModelLane>, &'static str> {
        let value = self.get_json("/api/v1/appliance/stats").map_err(|_| "caduceus-model-lanes-upstream-refused")?;
        let lanes = value.get("model_lanes").cloned().ok_or("caduceus-model-lanes-missing")?;
        serde_json::from_value(lanes).map_err(|_| "caduceus-model-lanes-invalid")
    }

    fn attendance_open(&self, pin: &str, document: &str) -> AttendanceCall {
        self.call(
            AttendanceOperation::Open,
            serde_json::json!({"pin": pin, "documentId": document, "documentIncarnation": document}),
        )
    }

    fn attendance_validate(&self, attendance: &AttendanceProof, document: &str) -> AttendanceCall {
        self.call(
            AttendanceOperation::Validate,
            serde_json::json!({"attendance": attendance.expose(), "documentId": document, "documentIncarnation": document}),
        )
    }

    fn attendance_invalidate(
        &self,
        attendance: &AttendanceProof,
        document: &str,
    ) -> AttendanceCall {
        self.call(
            AttendanceOperation::Invalidate,
            serde_json::json!({"attendance": attendance.expose(), "documentId": document, "documentIncarnation": document}),
        )
    }

    fn call(&self, operation: AttendanceOperation, body: serde_json::Value) -> AttendanceCall {
        let Ok(encoded) = serde_json::to_vec(&body) else {
            return AttendanceCall::refused(0, "caduceus-attendance-request-invalid");
        };
        if encoded.len() > CADUCEUS_ACCESS_MAX_REQUEST {
            return AttendanceCall::refused(0, "caduceus-attendance-request-invalid");
        }
        let (status, response) = match caduceus_raw_request(
            "POST",
            operation.path(),
            Some(&encoded),
            CADUCEUS_ACCESS_TIMEOUT,
            CADUCEUS_MAX_COMMAND_RESPONSE_BODY,
        ) {
            Ok(result) => result,
            Err(CaduceusTransportError::Connect(error)) => return AttendanceCall::refused(0, attendance_io_code("connect", &error)),
            Err(CaduceusTransportError::Io(error)) => {
                return AttendanceCall::refused(0, attendance_io_code("io", &error));
            }
            Err(CaduceusTransportError::Framing) => return AttendanceCall::refused(0, "caduceus-attendance-bad-receipt"),
        };
        parse_attendance_response(operation, status, &response)
    }

}

fn attendance_io_code(stage: &str, error: &std::io::Error) -> &'static str {
    match error.kind() {
        std::io::ErrorKind::ConnectionRefused | std::io::ErrorKind::NotFound if stage == "connect" => {
            "caduceus-attendance-connect-refused"
        }
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => {
            "caduceus-attendance-timeout"
        }
        _ if stage == "connect" => "caduceus-attendance-connect-failed",
        _ => "caduceus-attendance-write-failed",
    }
}

fn parse_attendance_response(operation: AttendanceOperation, status: u16, body: &[u8]) -> AttendanceCall {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(body) else {
        return AttendanceCall::refused(status, "caduceus-attendance-bad-receipt");
    };
    let Some(object) = value.as_object() else { return AttendanceCall::refused(status, "caduceus-attendance-bad-receipt"); };
    let ok = object.get("ok").and_then(serde_json::Value::as_bool).unwrap_or(false);
    let code = object.get("first_missing_signal").or_else(|| object.get("firstMissingSignal")).or_else(|| object.get("code")).and_then(serde_json::Value::as_str).unwrap_or(if ok { "none" } else { "caduceus-attendance-refused" });
    if !(200..300).contains(&status) || !ok { return AttendanceCall::refused(status, code); }
    let proof = operation.returns_proof().then(|| object.get("attendance").or_else(|| object.get("proof")).and_then(serde_json::Value::as_str).and_then(AttendanceProof::parse));
    if matches!(proof, Some(None)) { return AttendanceCall::refused(status, "caduceus-attendance-bad-receipt"); }
    AttendanceCall { ok: true, status, code: "none".to_string(), proof: proof.flatten() }
}

fn document_incarnation_from_headers(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get("x-caduceus-document")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| opaque_document(value))
        .map(ToOwned::to_owned)
}

fn attendance_from_headers(headers: &axum::http::HeaderMap) -> Option<AttendanceProof> {
    headers
        .get("x-caduceus-attendance")
        .and_then(|value| value.to_str().ok())
        .and_then(AttendanceProof::parse)
}

fn opaque_document(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn json_content_type(headers: &axum::http::HeaderMap) -> bool {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(str::trim)
        .is_some_and(|value| {
            value.eq_ignore_ascii_case("application/json")
                || value.to_ascii_lowercase().ends_with("+json")
        })
}

fn same_origin_state_change(headers: &axum::http::HeaderMap) -> bool {
    let Some(origin) = headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
    else {
        return false;
    };
    let Some((scheme, authority)) = origin.split_once("://") else {
        return false;
    };
    if !matches!(scheme, "http" | "https")
        || authority.is_empty()
        || authority.contains(['/', '?', '#', '@'])
    {
        return false;
    }
    let host = headers
        .get("x-forwarded-host")
        .or_else(|| headers.get(header::HOST))
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim);
    let Some(host) = host else {
        return false;
    };
    let normalized_origin = normalized_authority(authority, scheme);
    let normalized_host = normalized_authority(host, scheme);
    if normalized_origin.is_none() || normalized_origin != normalized_host {
        return false;
    }
    headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .is_none_or(|proto| proto.eq_ignore_ascii_case(scheme))
}

fn normalized_authority(raw: &str, scheme: &str) -> Option<String> {
    let raw = raw.trim().to_ascii_lowercase();
    if raw.is_empty() || raw.contains(['/', '?', '#', '@']) || raw.starts_with(':') {
        return None;
    }
    let default_port = if scheme == "https" { 443 } else { 80 };
    let (host, port) = match raw.rsplit_once(':') {
        Some((host, port)) if !host.is_empty() => match port.parse::<u16>() {
            Ok(port) => (host, port),
            Err(_) => (raw.as_str(), default_port),
        },
        _ => (raw.as_str(), default_port),
    };
    (!host.is_empty()).then(|| format!("{host}:{port}"))
}

fn attendance_failure_status(call: &AttendanceCall) -> StatusCode {
    if call.ok {
        StatusCode::OK
    } else if matches!(
        call.code.as_str(),
        "caduceus-attendance-refused"
            | "caduceus-attendance-pin-refused"
            | "caduceus-attendance-not-current"
            | "caduceus-attendance-invalid"
            | "caduceus-attendance-required"
            | "caduceus-stale-incarnation"
            | "caduceus-attendance-stale-incarnation"
    ) {
        StatusCode::UNAUTHORIZED
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}

fn attendance_projection(call: AttendanceCall) -> serde_json::Value {
    serde_json::json!({
        "schema": "arcadia.caduceus.attendance.projection.v1",
        "ok": call.ok,
        "admin": call.ok,
        "attendance": call.proof.as_ref().map(AttendanceProof::expose),
        "first_missing_signal": call.code.clone(),
        "firstMissingSignal": call.code,
    })
}

fn attendance_refusal(status: StatusCode, code: &str) -> Response {
    (
        status,
        Json(serde_json::json!({
            "schema": "arcadia.caduceus.attendance.projection.v1",
            "ok": false,
            "admin": false,
            "firstMissingSignal": safe_access_code(code),
            "first_missing_signal": safe_access_code(code),
        })),
    )
        .into_response()
}

async fn caduceus_attendance_open_route(
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    if !same_origin_state_change(&headers) {
        return attendance_refusal(StatusCode::FORBIDDEN, "caduceus-attendance-origin-refused");
    }
    if !json_content_type(&headers) || body.len() > CADUCEUS_ACCESS_MAX_REQUEST {
        return attendance_refusal(
            StatusCode::BAD_REQUEST,
            "caduceus-attendance-request-invalid",
        );
    }
    let Ok(body) = serde_json::from_slice::<serde_json::Value>(&body) else {
        return attendance_refusal(
            StatusCode::BAD_REQUEST,
            "caduceus-attendance-request-invalid",
        );
    };
    let Some(pin) = body
        .get("pin")
        .and_then(serde_json::Value::as_str)
        .filter(|pin| !pin.is_empty() && pin.len() <= 256)
    else {
        return attendance_refusal(StatusCode::BAD_REQUEST, "caduceus-attendance-pin-required");
    };
    let Some(document) = document_incarnation_from_headers(&headers) else {
        return attendance_refusal(
            StatusCode::BAD_REQUEST,
            "caduceus-attendance-document-required",
        );
    };
    let pin = pin.to_string();
    let call = tokio::task::spawn_blocking(move || CaduceusAccessClient::default().attendance_open(&pin, &document))
        .await.unwrap_or_else(|_| AttendanceCall::refused(503, "caduceus-attendance-worker-failed"));
    let status = attendance_failure_status(&call);
    (status, Json(attendance_projection(call))).into_response()
}

async fn caduceus_attendance_validate_route(headers: axum::http::HeaderMap) -> Response {
    let Some(document) = document_incarnation_from_headers(&headers) else {
        return attendance_refusal(
            StatusCode::BAD_REQUEST,
            "caduceus-attendance-document-required",
        );
    };
    let Some(attendance) = attendance_from_headers(&headers) else {
        return attendance_refusal(StatusCode::UNAUTHORIZED, "caduceus-attendance-required");
    };
    let call = tokio::task::spawn_blocking(move || CaduceusAccessClient::default().attendance_validate(&attendance, &document))
        .await.unwrap_or_else(|_| AttendanceCall::refused(503, "caduceus-attendance-worker-failed"));
    let status = attendance_failure_status(&call);
    (status, Json(attendance_projection(call))).into_response()
}

async fn caduceus_attendance_invalidate_route(headers: axum::http::HeaderMap) -> Response {
    let Some(document) = document_incarnation_from_headers(&headers) else {
        return attendance_refusal(
            StatusCode::BAD_REQUEST,
            "caduceus-attendance-document-required",
        );
    };
    let Some(attendance) = attendance_from_headers(&headers) else {
        return attendance_refusal(StatusCode::UNAUTHORIZED, "caduceus-attendance-required");
    };
    attendance_cache_bust(&document, &attendance);
    let call = tokio::task::spawn_blocking(move || CaduceusAccessClient::default().attendance_invalidate(&attendance, &document))
        .await.unwrap_or_else(|_| AttendanceCall::refused(503, "caduceus-attendance-worker-failed"));
    let status = attendance_failure_status(&call);
    (status, Json(attendance_projection(call))).into_response()
}

// Caduceus keeps custody of every administrative action. Arcadia validates the
// current document attendance before forwarding the same document/proof pair to
// Caduceus; neither value is logged or persisted here.
fn caduceus_attended_json_call(
    path: &'static str,
    headers: &axum::http::HeaderMap,
    body: serde_json::Value,
) -> Result<serde_json::Value, AttendanceCall> {
    let document = document_incarnation_from_headers(headers)
        .ok_or_else(|| AttendanceCall::refused(400, "caduceus-attendance-document-required"))?;
    let attendance = attendance_from_headers(headers)
        .ok_or_else(|| AttendanceCall::refused(401, "caduceus-attendance-required"))?;
    let client = CaduceusAccessClient::default();
    let validation = client.attendance_validate(&attendance, &document);
    if !validation.ok {
        return Err(validation);
    }
    let flags = serde_json::json!({"exousia": {"attendance": attendance.expose(), "documentId": document, "documentIncarnation": document}});
    let value = client
        .post_json_with_timeout_and_flags(path, body, CADUCEUS_ACCESS_TIMEOUT, flags)
        .map_err(|error| AttendanceCall::refused(503, error))?;
    let status = 200;
    let ok = value
        .get("ok")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    if !(200..300).contains(&status) || !ok {
        let code = value
            .get("first_missing_signal")
            .or_else(|| value.get("firstMissingSignal"))
            .or_else(|| value.get("code"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("caduceus-action-refused");
        return Err(AttendanceCall::refused(status, code));
    }
    Ok(value)
}

fn caduceus_action_response(result: Result<serde_json::Value, AttendanceCall>) -> Response {
    match result {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(call) => attendance_refusal(attendance_failure_status(&call), &call.code),
    }
}

async fn caduceus_pin_access_route(
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Response {
    caduceus_action_response(caduceus_attended_json_call(
        "/api/v1/exousia/change-pin",
        &headers,
        body,
    ))
}

async fn caduceus_pin_change_route(
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Response {
    caduceus_action_response(caduceus_attended_json_call(
        "/api/v1/exousia/change-pin",
        &headers,
        body,
    ))
}

async fn caduceus_pin_reset_route(
    headers: axum::http::HeaderMap,
    Json(_body): Json<serde_json::Value>,
) -> Response {
    caduceus_action_response(caduceus_attended_json_call(
        "/api/v1/exousia/change-pin",
        &headers,
        serde_json::json!({"action":"reset-default"}),
    ))
}

async fn caduceus_vault_unlock_route(
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Response {
    caduceus_action_response(caduceus_attended_json_call(
        "/api/v1/storage/vault/unlock",
        &headers,
        body,
    ))
}

async fn caduceus_vault_auto_decrypt_route(
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Response {
    caduceus_action_response(caduceus_attended_json_call(
        "/api/v1/storage/vault/auto-decrypt",
        &headers,
        body,
    ))
}
