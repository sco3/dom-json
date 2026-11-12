# Dynamic JSON Parsing: Go, Rust & Goose

A benchmark comparing dynamic (schemaless) JSON parsing across **Go**, **Rust**, and **Goose**. Each implementation
parses an untyped JSON payload 1,000,000 times and extracts 4 fields (`service`, `version`, nested `metrics.latency_ms`,
and `owner` presence).

## Input Payload

```json
{
  "service": "auth-api",
  "version": 2,
  "metrics": {
    "latency_ms": 15.4
  },
  "tags": [
    "core",
    "v2"
  ]
}
```

---

## Code Comparison

### Go (`main.go`)

Uses `encoding/json` into `map[string]any` with runtime type assertions:

### Rust (`src/main.rs`)

Uses `serde_json::Value` with chained indexing and accessor methods:

### Goose (`main.goose`)

Uses Goose's native `json` module with reusable AST buffers across iterations:

---

## Benchmark Results (1,000,000 iterations)

| Language  | Approach                                 | Time          | Relative Speed     |
|:----------|:-----------------------------------------|:--------------|:-------------------|
| **Rust**  | `serde_json::Value` (release profile)    | **~570 ms**   | **1.0x (fastest)** |
| **Goose** | Standard `json` module with buffer reuse | **~1,170 ms** | **~2.0x**          |
| **Go**    | `encoding/json` (`map[string]any`)       | **~3,800 ms** | **~6.6x**          |

---

## How to Run

Using Just:

```bash
just all     # Run all benchmarks
just go      # Run Go (go run main.go)
just rust    # Run Rust (cargo run --release)
just goose   # Run Goose (goose run main.goose)
```

~~~