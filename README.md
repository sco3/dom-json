# Dynamic JSON Parsing: Go, Rust & Goose

A benchmark comparing dynamic (schemaless) JSON parsing across **Go**, **Rust**, and **Goose**. Each implementation parses an untyped JSON payload 1,000,000 times and extracts 4 fields (`service`, `version`, nested `metrics.latency_ms`, and `owner` presence).

## Input Payload

```json
{
  "service": "auth-api",
  "version": 2,
  "metrics": { "latency_ms": 15.4 },
  "tags": ["core", "v2"]
}
```

---

## Code Comparison

### Go (`main.go`)
Uses `encoding/json` into `map[string]any` with runtime type assertions:
```go
func parse(data []byte) result {
    var m map[string]any
    json.Unmarshal(data, &m)

    res := result{}
    if svc, ok := m["service"].(string); ok { res.svc = svc }
    if ver, ok := m["version"].(float64); ok { res.ver = int(ver) }
    if metrics, ok := m["metrics"].(map[string]any); ok {
        if lat, ok := metrics["latency_ms"].(float64); ok { res.latency = lat }
    }
    _, res.owner = m["owner"]
    return res
}
```

### Rust (`src/main.rs`)
Uses `serde_json::Value` with chained indexing and accessor methods:
```rust
fn parse(data: &str) -> Result {
    let v: Value = serde_json::from_str(data).unwrap();
    Result {
        svc: v["service"].as_str().unwrap_or("").to_string(),
        ver: v["version"].as_i64().unwrap_or(0),
        latency: v["metrics"]["latency_ms"].as_f64().unwrap_or(0.0),
        owner: !v["owner"].is_null(),
    }
}
```

### Goose (`main.goose`)
Uses Goose's native `json` module with reusable AST buffers across iterations:
```goose
fn parse(text: u8[:], doc: json::Node[>..]&, decoded: u8[>..]&) -> Result {
    let root, err = json::parse(text, doc, decoded);
    guard root else { abort(err); }

    var res = Result { svc: "", ver: 0, latency: 0.0, owner: false };
    if let svc, ok = json::as_string(json::member(root, "service")); ok { res.svc = svc; }
    if let ver, ok = json::as_i64(json::member(root, "version")); ok { res.ver = ver; }
    let metrics = json::member(root, "metrics");
    if let lat, ok = json::as_f64(json::member(metrics, "latency_ms")); ok { res.latency = lat; }
    if json::member(root, "owner") { res.owner = true; }
    return res;
}
```

---

## Benchmark Results (1,000,000 iterations)

| Language | Approach | Time | Relative Speed |
| :--- | :--- | :--- | :--- |
| **Rust** | `serde_json::Value` (release profile) | **~570 ms** | **1.0x (fastest)** |
| **Goose** | Standard `json` module with buffer reuse | **~1,170 ms** | **~2.0x** |
| **Go** | `encoding/json` (`map[string]any`) | **~3,800 ms** | **~6.6x** |

---

## How to Run

Using [Just](https://github.com/casey/just):

```bash
just all     # Run all benchmarks
just go      # Run Go (go run main.go)
just rust    # Run Rust (cargo run --release)
just goose   # Run Goose (goose run main.goose)
```
