use serde_json::Value;
use std::time::Instant;

#[derive(Debug)]
#[allow(dead_code)]
struct Result {
    svc: String,
    ver: i64,
    latency: f64,
    owner: bool,
}

fn parse(data: &str) -> Result {
    let v: Value = serde_json::from_str(data).unwrap();
    Result {
        svc: v["service"].as_str().unwrap_or("").to_string(),
        ver: v["version"].as_i64().unwrap_or(0),
        latency: v["metrics"]["latency_ms"].as_f64().unwrap_or(0.0),
        owner: !v["owner"].is_null(),
    }
}

fn main() {
    let data = r#"{ "service": "auth-api", "version": 2, "metrics": {"latency_ms": 15.4}, "tags": ["core", "v2"] }"#;
    let start = Instant::now();
    let mut r = Result {
        svc: String::new(),
        ver: 0,
        latency: 0.0,
        owner: false,
    };
    for _ in 0..1_000_000 {
        r = parse(data);
    }
    println!("Took: {:?} result: {:?}", start.elapsed(), r);
}
