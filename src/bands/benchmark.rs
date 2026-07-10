use std::time::Instant;

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
pub struct BenchmarkMeasure {
    pub wall_ms: u128,
    pub process_rss_kib: Option<u64>,
    pub process_rss_delta_kib: Option<i64>,
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
    pub cost: Option<BenchmarkMeasure>,
    pub organ_cost: Option<BenchmarkMeasure>,
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
                cost: None,
                organ_cost: None,
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
        cost: None,
        organ_cost: None,
    }
}

fn benchmark_sample_request(
    request: BenchmarkSampleRequest,
) -> Result<BenchmarkSampleRequest, BenchmarkReceipt> {
    let scope = request.scope.trim().to_string();
    let reference = request.reference.trim().to_string();
    let load = request.load.trim().to_string();
    let window = request.window.trim().to_string();
    if scope.is_empty() || reference.is_empty() || load.is_empty() || window.is_empty() {
        let mut receipt = benchmark_receipt(
            "sample-receipt",
            "refused",
            scope,
            "A benchmark sample must declare scope, reference, load, and window.",
        );
        receipt.reference = (!reference.is_empty()).then_some(reference);
        receipt.load = (!load.is_empty()).then_some(load);
        receipt.window = (!window.is_empty()).then_some(window);
        return Err(receipt);
    }
    if load != "idle-truth" {
        let mut receipt = benchmark_receipt(
            "sample-receipt",
            "refused",
            scope,
            "Slice 2 recognizes only the declared idle-truth load condition.",
        );
        receipt.reference = Some(reference);
        receipt.load = Some(load);
        receipt.window = Some(window);
        return Err(receipt);
    }
    Ok(BenchmarkSampleRequest {
        scope,
        reference,
        load,
        window,
    })
}

fn process_rss_kib() -> Option<u64> {
    let statm = fs::read_to_string("/proc/self/statm").ok()?;
    let resident_pages = statm.split_whitespace().nth(1)?.parse::<u64>().ok()?;
    Some(resident_pages.saturating_mul(4))
}

fn rss_delta_kib(before: Option<u64>, after: Option<u64>) -> Option<i64> {
    Some(after? as i64 - before? as i64)
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
    let request = match benchmark_sample_request(request) {
        Ok(request) => request,
        Err(receipt) => return (StatusCode::BAD_REQUEST, Json(receipt)),
    };
    let aggregate = benchmark_status();
    if aggregate.state == "asleep" {
        let mut receipt = benchmark_receipt(
            "sample-receipt",
            "refused",
            request.scope,
            "Benchmark sampling is refused while the organ is asleep; compose a named scope first.",
        );
        receipt.reference = Some(request.reference);
        receipt.load = Some(request.load);
        receipt.window = Some(request.window);
        return (StatusCode::CONFLICT, Json(receipt));
    }

    let probe_started = Instant::now();
    let subject_started = Instant::now();
    let subject_rss_kib = process_rss_kib();
    let cost = BenchmarkMeasure {
        wall_ms: subject_started.elapsed().as_millis(),
        process_rss_kib: subject_rss_kib,
        process_rss_delta_kib: None,
    };
    let probe_rss_kib = process_rss_kib();
    let organ_cost = BenchmarkMeasure {
        wall_ms: probe_started.elapsed().as_millis(),
        process_rss_kib: probe_rss_kib,
        process_rss_delta_kib: rss_delta_kib(subject_rss_kib, probe_rss_kib),
    };
    let mut receipt = benchmark_receipt(
        "benchmark-receipt",
        "receipted",
        request.scope,
        "Single-shot idle-truth probe completed with host-local wall and process RSS measures; no sampler remains.",
    );
    receipt.reference = Some(request.reference);
    receipt.load = Some(request.load);
    receipt.window = Some(request.window);
    receipt.cost = Some(cost);
    receipt.organ_cost = Some(organ_cost);
    let mut aggregate = benchmark_aggregate().lock().expect("benchmark aggregate lock");
    aggregate.last_receipt = Some(receipt.clone());
    (StatusCode::OK, Json(receipt))
}
