#[test]
fn benchmark_organ_is_asleep_by_default_and_cold() {
    let aggregate = BenchmarkAggregate::asleep();

    assert_eq!(aggregate.schema, "arcadia.benchmark.organ.v1");
    assert_eq!(aggregate.state, "asleep");
    assert!(aggregate.scope.is_none());
    assert!(aggregate.last_receipt.is_none());
}

#[tokio::test]
async fn benchmark_routes_compose_then_refuse_probe_then_uncompose() {
    let scope = "src/bands/status-core".to_string();
    let compose = benchmark_compose_route(Json(BenchmarkScopeRequest {
        scope: scope.clone(),
    }))
    .await
    .expect("compose must accept named scope")
    .0;

    assert_eq!(compose.kind, "compose-receipt");
    assert_eq!(compose.outcome, "lit");
    assert_eq!(benchmark_status().scope.as_deref(), Some(scope.as_str()));

    let (status, sample) = benchmark_sample_route(Json(BenchmarkSampleRequest {
        scope: scope.clone(),
        reference: "arcadia-idle".to_string(),
        load: "idle-truth".to_string(),
        window: "single-shot".to_string(),
    }))
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(sample.0.outcome, "refused");
    assert_eq!(sample.0.load.as_deref(), Some("idle-truth"));

    let uncompose = benchmark_uncompose_route(Json(BenchmarkScopeRequest { scope }))
        .await
        .expect("uncompose must accept named scope")
        .0;
    assert_eq!(uncompose.kind, "uncompose-receipt");
    assert_eq!(benchmark_status().state, "asleep");
    assert!(benchmark_status().scope.is_none());
}

#[test]
fn benchmark_refuses_unnamed_scope() {
    let refusal = benchmark_scope("   ".to_string()).expect_err("empty scope must refuse");

    assert_eq!(refusal.0, StatusCode::BAD_REQUEST);
    assert_eq!(refusal.1.0.outcome, "refused");
}
