//! API response monitoring: detect correlations and conservation laws in endpoint metrics.

use crackle_runtime::{CrackleTask, Kiln, ThermalProfile, TaskOutput};

struct ApiRequest {
    endpoint: String,
    latency_ms: f64,
    status: u16,
    response_bytes: f64,
}

impl CrackleTask for ApiRequest {
    type Output = f64;

    fn fire(&self) -> TaskOutput<Self::Output> {
        TaskOutput::new(
            self.latency_ms,
            vec![
                ("latency_ms".into(), self.latency_ms),
                ("response_bytes".into(), self.response_bytes),
                ("is_error".into(), if self.status >= 400 { 1.0 } else { 0.0 }),
                ("is_slow".into(), if self.latency_ms > 200.0 { 1.0 } else { 0.0 }),
            ],
        )
    }

    fn label(&self) -> String {
        self.endpoint.clone()
    }
}

fn main() {
    let mut kiln = Kiln::new(ThermalProfile::default());

    // /api/users — fast, small responses
    kiln.fire_and_record(ApiRequest { endpoint: "GET /api/users".into(), latency_ms: 45.0, status: 200, response_bytes: 1200.0 }).unwrap();
    kiln.fire_and_record(ApiRequest { endpoint: "GET /api/users".into(), latency_ms: 48.0, status: 200, response_bytes: 1150.0 }).unwrap();
    kiln.fire_and_record(ApiRequest { endpoint: "GET /api/users".into(), latency_ms: 52.0, status: 200, response_bytes: 1300.0 }).unwrap();

    // /api/reports — slow, large responses (correlated with bytes)
    kiln.fire_and_record(ApiRequest { endpoint: "GET /api/reports".into(), latency_ms: 350.0, status: 200, response_bytes: 55000.0 }).unwrap();
    kiln.fire_and_record(ApiRequest { endpoint: "GET /api/reports".into(), latency_ms: 380.0, status: 200, response_bytes: 62000.0 }).unwrap();

    // /api/health — tiny, always fast (conservation candidate)
    kiln.fire_and_record(ApiRequest { endpoint: "GET /api/health".into(), latency_ms: 3.0, status: 200, response_bytes: 15.0 }).unwrap();
    kiln.fire_and_record(ApiRequest { endpoint: "GET /api/health".into(), latency_ms: 4.0, status: 200, response_bytes: 15.0 }).unwrap();

    let patterns = kiln.cool();
    println!("API Monitoring — Pattern Report");
    println!("================================\n");
    println!("{} patterns detected:\n", patterns.len());

    for p in &patterns {
        println!("[{}] {}", p.kind(), p.description());
        println!("  confidence: {:.2}", p.confidence());
        if !p.involved_tasks().is_empty() {
            println!("  endpoints: {:?}", p.involved_tasks());
        }
        println!();
    }

    // Highlight correlations between latency and response size
    let correlations: Vec<_> = patterns.iter()
        .filter(|p| format!("{}", p.kind()) == "correlation")
        .collect();

    if !correlations.is_empty() {
        println!("📊 Latency and response size are correlated — consider pagination or caching.");
    }
}
