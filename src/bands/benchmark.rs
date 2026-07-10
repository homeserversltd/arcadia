#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchmarkAggregate {
    pub schema: &'static str,
    pub state: &'static str,
    pub scope: Option<String>,
    pub last_receipt: Option<BenchmarkReceipt>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchmarkReceipt {
    pub schema: &'static str,
    pub kind: &'static str,
    pub outcome: &'static str,
    pub scope: String,
    pub detail: String,
    pub reference: Option<String>,
    pub load: Option<String>,
    pub window: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BenchmarkScopeRequest {
    scope: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BenchmarkSampleRequest {
    scope: String,
    reference: String,
    load: String,
    window: String,
}

impl BenchmarkAggregate {
    fn asleep() -> Self {
        Self {
            schema: "arcadia.benchmark.organ.v1",
            state: "asleep",
            scope: None,
            last_receipt: None,
        }
    }
}

static BENCHMARK_AGGREGATE: OnceLock<Mutex<BenchmarkAggregate>> = OnceLock::new();

fn benchmark_aggregate() -> &'static Mutex<BenchmarkAggregate> {
    BENCHMARK_AGGREGATE.get_or_init(|| Mutex::new(BenchmarkAggregate::asleep()))
}

fn benchmark_status() -> BenchmarkAggregate {
    benchmark_aggregate()
        .lock()
        .expect("benchmark aggregate lock")
        .clone()
}

fn benchmark_scope(scope: String) -> Result<String, (StatusCode, Json<BenchmarkReceipt>)> {
    let scope = scope.trim().to_string();
    if scope.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(BenchmarkReceipt {
                schema: "arcadia.benchmark.receipt.v1",
                kind: "compose-receipt",
                outcome: "refused",
                scope,
                detail: "A benchmark compose request must name a scope.".to_string(),
                reference: None,
                load: None,
                window: None,
            }),
        ));
    }
    Ok(scope)
}

fn benchmark_receipt(
    kind: &'static str,
    outcome: &'static str,
    scope: String,
    detail: impl Into<String>,
) -> BenchmarkReceipt {
    BenchmarkReceipt {
        schema: "arcadia.benchmark.receipt.v1",
        kind,
        outcome,
        scope,
        detail: detail.into(),
        reference: None,
        load: None,
        window: None,
    }
}

async fn benchmark_status_route() -> Json<BenchmarkAggregate> {
    Json(benchmark_status())
}

async fn benchmark_compose_route(
    Json(request): Json<BenchmarkScopeRequest>,
) -> Result<Json<BenchmarkReceipt>, (StatusCode, Json<BenchmarkReceipt>)> {
    let scope = benchmark_scope(request.scope)?;
    let receipt = benchmark_receipt(
        "compose-receipt",
        "lit",
        scope.clone(),
        "Benchmark organ is lit for the named scope; no sampler has started.",
    );
    let mut aggregate = benchmark_aggregate().lock().expect("benchmark aggregate lock");
    aggregate.state = "lit";
    aggregate.scope = Some(scope);
    aggregate.last_receipt = Some(receipt.clone());
    Ok(Json(receipt))
}

async fn benchmark_uncompose_route(
    Json(request): Json<BenchmarkScopeRequest>,
) -> Result<Json<BenchmarkReceipt>, (StatusCode, Json<BenchmarkReceipt>)> {
    let scope = benchmark_scope(request.scope)?;
    let receipt = benchmark_receipt(
        "uncompose-receipt",
        "asleep",
        scope,
        "Benchmark organ is asleep; no sampler or background work remains.",
    );
    let mut aggregate = benchmark_aggregate().lock().expect("benchmark aggregate lock");
    aggregate.state = "asleep";
    aggregate.scope = None;
    aggregate.last_receipt = Some(receipt.clone());
    Ok(Json(receipt))
}

async fn benchmark_sample_route(
    Json(request): Json<BenchmarkSampleRequest>,
) -> (StatusCode, Json<BenchmarkReceipt>) {
    let aggregate = benchmark_status();
    let mut receipt = benchmark_receipt(
        "sample-receipt",
        "refused",
        request.scope,
        if aggregate.state == "asleep" {
            "Benchmark sampling is refused while the organ is asleep; compose a named scope first."
        } else {
            "Benchmark sampling is not implemented in slice 1; no probe or background work was started."
        },
    );
    receipt.reference = Some(request.reference);
    receipt.load = Some(request.load);
    receipt.window = Some(request.window);
    (StatusCode::CONFLICT, Json(receipt))
}
