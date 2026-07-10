#[test]
fn benchmark_organ_is_asleep_by_default_and_cold() {
    let aggregate = BenchmarkAggregate::asleep();

    assert_eq!(aggregate.schema, "arcadia.benchmark.organ.v1");
    assert_eq!(aggregate.state, "asleep");
    assert!(aggregate.scope.is_none());
    assert!(aggregate.last_receipt.is_none());
}

#[tokio::test]
async fn benchmark_refuses_sample_while_asleep_then_receipts_idle_truth_once_lit() {
    let scope = "src/bands/status-core".to_string();
    let request = || BenchmarkSampleRequest {
        scope: scope.clone(),
        reference: "arcadia-idle".to_string(),
        load: "idle-truth".to_string(),
        window: "single-shot".to_string(),
    };

    let (status, asleep) = benchmark_sample_route(Json(request())).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(asleep.0.outcome, "refused");

    let compose = benchmark_compose_route(Json(BenchmarkScopeRequest {
        scope: scope.clone(),
    }))
    .await
    .expect("compose must accept named scope")
    .0;
    assert_eq!(compose.kind, "compose-receipt");
    assert_eq!(compose.outcome, "lit");

    let (status, sample) = benchmark_sample_route(Json(request())).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(sample.0.kind, "benchmark-receipt");
    assert_eq!(sample.0.outcome, "receipted");
    assert_eq!(sample.0.load.as_deref(), Some("idle-truth"));
    assert!(sample.0.cost.is_some());
    assert!(sample.0.organ_cost.is_some());
    assert_eq!(benchmark_status().state, "lit");
    assert!(benchmark_status().last_receipt.is_some());

    let uncompose = benchmark_uncompose_route(Json(BenchmarkScopeRequest { scope }))
        .await
        .expect("uncompose must accept named scope")
        .0;
    assert_eq!(uncompose.kind, "uncompose-receipt");
    assert_eq!(benchmark_status().state, "asleep");
    assert!(benchmark_status().scope.is_none());
}

#[tokio::test]
async fn benchmark_refuses_samples_without_every_declared_field() {
    let (status, receipt) = benchmark_sample_route(Json(BenchmarkSampleRequest {
        scope: "src/bands/status-core".to_string(),
        reference: " ".to_string(),
        load: "idle-truth".to_string(),
        window: "single-shot".to_string(),
    }))
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(receipt.0.outcome, "refused");
    assert!(receipt.0.detail.contains("scope, reference, load, and window"));
}

#[test]
fn benchmark_refuses_unnamed_scope() {
    let refusal = benchmark_scope("   ".to_string()).expect_err("empty scope must refuse");

    assert_eq!(refusal.0, StatusCode::BAD_REQUEST);
    assert_eq!(refusal.1.0.outcome, "refused");
}
