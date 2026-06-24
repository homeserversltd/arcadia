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

async fn caduceus_json_proxy(path: &'static str) -> impl IntoResponse {
    match caduceus_fetch_json(path) {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(signal) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({
                "schema": "arcadia.caduceus.proxy.error.v1",
                "ok": false,
                "path": path,
                "first_missing_signal": signal,
            })),
        )
            .into_response(),
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