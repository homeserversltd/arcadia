async fn provider_keys_status() -> impl IntoResponse {
    caduceus_json_proxy("/api/v1/games/provider-keys").await
}

async fn save_provider_keys(Json(body): Json<serde_json::Value>) -> impl IntoResponse {
    caduceus_json_post_proxy("/api/v1/games/provider-keys", body)
}
