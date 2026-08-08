async fn css() -> Response {
    asset_owned(
        format!("{}\n{}\n{}\n{}", THEME_CSS, UX_CSS, APP_CSS, VIEWPORT_CSS),
        "text/css; charset=utf-8",
    )
}
async fn js() -> Response {
    asset_owned(
        format!("{}\n{}", THEME_JS, APP_JS),
        "application/javascript; charset=utf-8",
    )
}

async fn indra_observation_js() -> Response {
    asset_owned(
        INDRA_OBSERVATION_JS.to_string(),
        "application/javascript; charset=utf-8",
    )
}

async fn vendor_chart_js() -> Response {
    asset_owned(VENDOR_CHART_JS.to_string(), "application/javascript; charset=utf-8")
}
async fn not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "not found")
}

fn asset_owned(body: String, content_type: &'static str) -> Response {
    let mut response = body.into_response();
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    response
}
