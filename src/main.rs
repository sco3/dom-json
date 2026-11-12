use serde_json::Value;

fn main() {
    let data = r#"{
        "service": "auth-api",
        "version": 2,
        "metrics": {"latency_ms": 15.4},
        "tags": ["core", "v2"]
    }"#;

    let v: Value = serde_json::from_str(data).unwrap();

    // 1. service
    if let Some(svc) = v["service"].as_str() {
        println!("Service: {}", svc);
    }

    // 2. version
    if let Some(ver) = v["version"].as_i64() {
        println!("Version: {}", ver);
    }

    // 3. latency_ms (safe chain)
    if let Some(lat) = v["metrics"]["latency_ms"].as_f64() {
        println!("Latency: {}", lat);
    }

    // 4. owner (Value::Null)
    if v["owner"].is_null() {
        println!("Owner is missing (expected)");
    }
}