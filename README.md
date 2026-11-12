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

## Measured Benchmark Results (1,000,000 iterations)

All timings measured directly on an AMD Ryzen 5 4500U (Linux x86_64, Go 1.27.2, Rust 1.85, Goose 0.4):

| Language | Implementation / Approach | Measured Time | Relative Speed |
|:---|:---|:---|:---|
| **Rust** | `serde_json::Value` (release profile, `src/main.rs`) | **604.52 ms** | **1.00x (baseline)** |
| **Go** | Stdlib `encoding/json/jsontext.Decoder` (buffer reuse, `main_test.go`) | **671.59 ms** | **1.11x** |
| **Go** | Stdlib `encoding/json/v2` (typed struct, `main_test.go`) | **951.16 ms** | **1.57x** |
| **Go** | Stdlib `encoding/json` v1 (typed struct, `main_test.go`) | **1,168.35 ms** | **1.93x** |
| **Goose** | Native `json` module with reusable AST buffers (`main.goose`) | **1,179.00 ms** | **1.95x** |
| **Go** | Stdlib `encoding/json/v2` with `clear(m)` map reuse (`main_test.go`) | **2,306.89 ms** | **3.82x** |
| **Go** | Stdlib `encoding/json/v2` `map[string]any` (`main_test.go`) | **2,514.25 ms** | **4.16x** |
| **Go** | Stdlib `encoding/json` v1 with `clear(m)` map reuse (`main_test.go`) | **3,643.54 ms** | **6.03x** |
| **Go** | Stdlib `encoding/json` v1 `map[string]any` (baseline, `main.go`) | **3,788.12 ms** | **6.27x slower** |

---

## Go Micro-Benchmarks & Allocation Profile

Measured via `go test -bench=. -benchmem -run=^$`:

| Benchmark Routine | Speed (`ns/op`) | Memory (`B/op`) | Allocs (`allocs/op`) |
|:---|:---:|:---:|:---:|
| `BenchmarkV2_JsontextDecoder_Reused` | **663.9 ns/op** | 8 B/op | 1 allocs/op |
| `BenchmarkV2_Struct` | **944.4 ns/op** | 64 B/op | 1 allocs/op |
| `BenchmarkV1_Struct` | **1,165.0 ns/op** | 64 B/op | 1 allocs/op |
| `BenchmarkV2_JsontextDecoder_Fresh` | **1,361.0 ns/op** | 736 B/op | 17 allocs/op |
| `BenchmarkV2_MapReused` | **2,312.0 ns/op** | 528 B/op | 14 allocs/op |
| `BenchmarkV2_Map` | **2,517.0 ns/op** | 864 B/op | 16 allocs/op |
| `BenchmarkV1_Decoder` | **2,609.0 ns/op** | 336 B/op | 22 allocs/op |
| `BenchmarkV1_MapReused` | **3,641.0 ns/op** | 640 B/op | 22 allocs/op |
| `BenchmarkV1_Map` (current `main.go`) | **3,817.0 ns/op** | 976 B/op | 24 allocs/op |

---

## Key Findings

### 1. Why Go Baseline (`encoding/json` v1 `map[string]any`) is ~6.3x Slower
- **Allocation overhead:** Every parse allocates top-level and nested maps, a slice for `"tags"`, and boxes primitive types (`float64`, `string`) into `any` interface values (24 heap allocations, 976 bytes per iteration). Over 1,000,000 iterations, this produces **24 million heap allocations** (~976 MB total allocated memory), saturating the GC.
- **Reflection:** Standard `encoding/json` v1 heavily relies on runtime reflection when unmarshaling into untyped structures.

### 2. Matching Rust Performance with Go Standard Library (`encoding/json/jsontext`)
- Rust's `serde_json::Value` parses into a compact, optimized enum tree without GC overhead (**604.52 ms**).
- Goose reuses internal AST buffers across iterations (**1,179 ms**).
- By applying the same buffer reuse pattern with Go's standard library `encoding/json/jsontext.Decoder` (`dec.Reset(buf)`), heap allocations drop to **1 alloc/op** (8 bytes) and parsing time drops to **671.59 ms** (**1.11x** of Rust).

### 3. Impact of Typed Structs
- If payload fields are known and represented as a Go `struct`, standard library unmarshaling drops allocations to 1 alloc/op (64 bytes), running in **951.16 ms** with `encoding/json/v2` and **1,168.35 ms** with v1.

### 4. Standard Library v2 vs v1 for Dynamic Maps
- When parsing into `map[string]any`, `encoding/json/v2` cuts allocations from 24 to 16 allocs/op and lowers execution time from **3,788 ms** to **2,514 ms** (~1.5x speedup).

---

## How to Run

Using Just:

```bash
just all         # Run original Go, Rust, and Goose benchmarks
just go          # Run Go baseline (go run main.go)
just rust        # Run Rust (cargo run --release)
just goose       # Run Goose (goose run main.goose)
just compare-go  # Run 1,000,000 iterations comparison across all Go routines
just bench-go    # Run Go micro-benchmarks with memory allocations
```
