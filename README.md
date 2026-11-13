# Dynamic JSON vs. Streaming JSON Parsing: Rust, Go & Goose

Benchmark comparing **Dynamic JSON Parsing** and **Streaming JSON Parsing** in **Rust**, **Go**, **Goose**. 

Code parses JSON payload **1,000,000 times**.

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

## 1. Dynamic JSON Parsing


| Language | Approach | Implementation | Measured Time | Relative Speed |
|:---|:---|:---|:---|:---|
| **Goose** | `json::Node` | Native AST DOM with buffer reuse, `goose build` (`main.goose`) | **236.00 ms** | **1.00x (fastest)** |
| **Rust** | `serde_json::Value` | Dynamic DOM enum tree (`src/main.rs`) | **587.70 ms** | **2.49x** |
| **Go** | `encoding/json/v2` | `map[string]any` with `clear(m)` reuse (`main_test.go`) | **2,364.44 ms** | **10.02x** |


---

## 2. Streaming JSON Parsing (Token-by-Token / Visitor)


| Language | Approach | Implementation | Measured Time | Relative Speed |
|:---|:---|:---|:---|:---|
| **Goose** | `json::reader` Token Stream | `json::each_member` cursor over reused `scratch`, `goose build` (`streaming.goose`) | **186.00 ms** | **1.00x (fastest)** |
| **Rust** | Zero-Copy Token Scanner | `TokenScanner` byte tokenizer, AWS-Smithy style (`src/main.rs`) | **213.66 ms** | **1.15x** |
| **Rust** | `serde` Streaming Visitor | `Deserializer::deserialize_map` with `Visitor` (`src/main.rs`) | **218.25 ms** | **1.17x** |
| **Go** | `encoding/json/jsontext` | `jsontext.Decoder` with buffer reset (`main_test.go`) | **671.94 ms** | **3.61x** |


---

## 3. Reference: Static Schema (Typed Struct Deserialization)

Unmarshaling directly into a statically typed Go struct bypasses untyped map allocations:

| Language | Approach | Implementation | Measured Time |
|:---|:---|:---|:---:|
| **Go** | `encoding/json/v2` | `payloadStruct` unmarshal (`main_test.go`) | **966.23 ms** |

---

## 4. Go Micro-Benchmarks & Profile

Measured via `go test -bench=. -benchmem -count=3 -run=^$` (median of 3 runs):

| Category | Benchmark Routine | Speed (`ns/op`) | Memory (`B/op`) | Allocs (`allocs/op`) |
|:---|:---|:---:|:---:|:---:|
| **Streaming** | `BenchmarkV2_JsontextDecoder_Reused` | **676.9 ns/op** | 8 B/op | 1 allocs/op |
| **Static Struct** | `BenchmarkV2_Struct` | **953.1 ns/op** | 64 B/op | 1 allocs/op |
| **Dynamic DOM** | `BenchmarkV2_MapReused` | **2,386.0 ns/op** | 528 B/op | 14 allocs/op |

---

## How to Run

Using Just:

```bash
just all          # Run original Go, Rust, and Goose benchmarks
just go           # Run Go example (go run main.go)
just rust         # Run Rust benchmarks: dynamic, streaming visitor, and token scanner (cargo run --release)
just goose        # Build and run Goose benchmarks as native binaries (goose build)
just compare-go   # Run 1,000,000 iterations comparison across all Go routines
just bench-go     # Run Go micro-benchmarks with memory allocations
```

Or run Goose directly:

```bash
goose build main.goose -o main-goose && ./main-goose                     # DOM benchmark (native)
goose build streaming.goose -o streaming-goose && ./streaming-goose     # Streaming benchmark (native)
```
