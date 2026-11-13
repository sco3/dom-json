# Dynamic JSON vs. Streaming JSON Parsing: Rust, Go & Goose

A benchmark comparing **Dynamic (Schemaless) JSON Parsing** and **Streaming (Token-by-Token) JSON Parsing** across **Rust**, **Go**, and **Goose**. 

Each benchmark parses an untyped JSON payload **1,000,000 times** and extracts 4 fields: `service`, `version`, nested `metrics.latency_ms`, and `owner` presence.

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

## 1. Dynamic (Schemaless) JSON Parsing (AST / DOM)

In this category, the parser has no compile-time schema. It parses the arbitrary, untyped JSON payload into an in-memory dynamic tree/object structure (AST / DOM), and the consumer navigates it dynamically (e.g. `doc["service"]`, `json::member(root, "service")`, `m["service"]`).

### Measured Results (1,000,000 iterations)

*All timings measured directly on an AMD Ryzen 5 4500U (Linux x86_64, Go 1.27.2, Rust 1.85, Goose 0.4):*

| Language | Approach | Implementation | Measured Time | Relative Speed |
|:---|:---|:---|:---|:---|
| **Rust** | `serde_json::Value` | Dynamic DOM enum tree (`src/main.rs`) | **580.87 ms** | **1.00x (fastest)** |
| **Goose** | `json::Node` | Native AST DOM with buffer reuse (`main.goose`) | **1,179.00 ms** | **2.03x** |
| **Go** | `encoding/json/v2` | `map[string]any` with `clear(m)` reuse (`main_test.go`) | **2,333.15 ms** | **4.02x** |
| **Go** | `encoding/json/v2` | `map[string]any` fresh allocations (`main_test.go`) | **2,536.16 ms** | **4.37x** |
| **Go** | `encoding/json` v1 | `map[string]any` with `clear(m)` reuse (`main_test.go`) | **3,655.20 ms** | **6.29x** |
| **Go** | `encoding/json` v1 | `map[string]any` baseline (`main.go`) | **3,788.12 ms** | **6.52x slower** |

### Why Go Dynamic Parsing is ~6.5x Slower
- **Memory allocations & GC pressure:** Unmarshaling an untyped payload into `map[string]any` allocates nested map buckets, slice headers for arrays (`"tags"`), and boxes primitive values (`float64`, `string`) into interface values (`any`). This performs **24 heap allocations (976 B/op)** per parse—totaling **24 million heap allocations** (~976 MB memory allocated) across 1,000,000 iterations, keeping the Go GC under continuous load.
- **Reflection:** Standard `encoding/json` v1 relies heavily on runtime reflection to decode into arbitrary interface types. `encoding/json/v2` optimizes reflection and reduces allocations from 24 to 16 allocs/op, improving time to **2,536 ms** (~1.5x speedup over v1).
- **Rust & Goose design:** Rust's `serde_json::Value` is a compact enum parsed with no garbage collector overhead. Goose's native `json` parser reuses internal node and decoding buffers (`doc`, `decoded`) across iterations, avoiding repeated allocations.

---

## 2. Streaming JSON Parsing (Token-by-Token / Visitor)

Streaming parsers do not construct an in-memory dynamic AST or DOM. Instead, they scan the byte stream token-by-token (or byte-by-byte), matching specific keys on the fly and skipping unneeded branches without allocating intermediate structures.

Comparing streaming parsers to dynamic AST parsers is fundamentally an apples-to-oranges comparison. As shown below, when Rust is also measured using streaming token parsing, Rust is **~3.1x faster than Go's standard streaming decoder**.

### Measured Results (1,000,000 iterations)

| Language | Approach | Implementation | Measured Time | Relative Speed |
|:---|:---|:---|:---|:---|
| **Rust** | Zero-Copy Token Scanner | `TokenScanner` byte tokenizer, AWS-Smithy style (`src/main.rs`) | **210.71 ms** | **1.00x (fastest)** |
| **Rust** | `serde` Streaming Visitor | `Deserializer::deserialize_map` with `Visitor` (`src/main.rs`) | **215.72 ms** | **1.02x** |
| **Go** | `encoding/json/jsontext` | `jsontext.Decoder` with buffer reset (`main_test.go`) | **669.11 ms** | **3.17x** |
| **Go** | `encoding/json/jsontext` | `jsontext.Decoder` fresh without buffer reset (`main_test.go`) | **1,361.00 ms** | **6.46x** |
| **Go** | `encoding/json` v1 | `json.NewDecoder` token streaming (`main_test.go`) | **2,609.00 ms** | **12.38x slower** |

### Streaming Analysis & Production Ecosystems
- **Rust Streaming (~210–216 ms):** By parsing tokens directly from bytes with zero heap allocation, Rust processes ~4.7 million JSON payloads per second per core.
  - **AWS Smithy (`aws-smithy-json`):** AWS utilizes this exact token-streaming model in the AWS SDK for Rust to parse wire protocols with zero allocations and minimal CPU overhead.
  - **SIMD Accelerators (`sonic-rs` / `simd-json`):** Libraries like `sonic-rs` take this further by utilizing AVX2/SSE/NEON vector instructions to scan and check tag existence at gigabytes per second without full deserialization.
- **Go Streaming (`jsontext.Decoder` ~669 ms):** Go 1.27's standard library `encoding/json/jsontext.Decoder` provides low-level token-streaming primitives. When resetting over a reused buffer (`dec.Reset(buf)`), it achieves 1 alloc/op (8 bytes), bringing Go from ~3.79s down to **669 ms**.

---

## 3. Reference: Static Schema (Typed Struct Deserialization)

When the schema is known at compile time, unmarshaling directly into a statically typed Go struct bypasses untyped map allocations:

| Language | Approach | Implementation | Measured Time |
|:---|:---|:---|:---:|
| **Go** | `encoding/json/v2` | `payloadStruct` unmarshal (`main_test.go`) | **956.75 ms** |
| **Go** | `encoding/json` v1 | `payloadStruct` unmarshal (`main_test.go`) | **1,173.32 ms** |

---

## 4. Go Micro-Benchmarks & Memory Allocation Profile

Measured via `go test -bench=. -benchmem -run=^$`:

| Category | Benchmark Routine | Speed (`ns/op`) | Memory (`B/op`) | Allocs (`allocs/op`) |
|:---|:---|:---:|:---:|:---:|
| **Streaming** | `BenchmarkV2_JsontextDecoder_Reused` | **663.9 ns/op** | 8 B/op | 1 allocs/op |
| **Streaming** | `BenchmarkV2_JsontextDecoder_Fresh` | **1,361.0 ns/op** | 736 B/op | 17 allocs/op |
| **Streaming** | `BenchmarkV1_Decoder` | **2,609.0 ns/op** | 336 B/op | 22 allocs/op |
| **Static Struct** | `BenchmarkV2_Struct` | **944.4 ns/op** | 64 B/op | 1 allocs/op |
| **Static Struct** | `BenchmarkV1_Struct` | **1,165.0 ns/op** | 64 B/op | 1 allocs/op |
| **Dynamic DOM** | `BenchmarkV2_MapReused` | **2,312.0 ns/op** | 528 B/op | 14 allocs/op |
| **Dynamic DOM** | `BenchmarkV2_Map` | **2,517.0 ns/op** | 864 B/op | 16 allocs/op |
| **Dynamic DOM** | `BenchmarkV1_MapReused` | **3,641.0 ns/op** | 640 B/op | 22 allocs/op |
| **Dynamic DOM** | `BenchmarkV1_Map` (current `main.go`) | **3,817.0 ns/op** | 976 B/op | 24 allocs/op |

---

## How to Run

Using Just:

```bash
just all         # Run original Go, Rust, and Goose benchmarks
just go          # Run Go baseline (go run main.go)
just rust        # Run Rust benchmarks: dynamic, streaming visitor, and token scanner (cargo run --release)
just goose       # Run Goose benchmark (goose run main.goose)
just compare-go  # Run 1,000,000 iterations comparison across all Go routines
just bench-go    # Run Go micro-benchmarks with memory allocations
```
