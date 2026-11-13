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

*All timings are medians of repeated runs measured directly on an AMD Ryzen 5 4500U (Linux x86_64, Go 1.27.2, Rust 1.85, Goose 0.4). Goose numbers are from optimized native binaries built with `goose build` (running the same benchmarks under the `goose run` interpreter is ~5x slower: 1,179 ms DOM / 927 ms streaming):*

| Language | Approach | Implementation | Measured Time | Relative Speed |
|:---|:---|:---|:---|:---|
| **Goose** | `json::Node` | Native AST DOM with buffer reuse, `goose build` (`main.goose`) | **236.00 ms** | **1.00x (fastest)** |
| **Rust** | `serde_json::Value` | Dynamic DOM enum tree (`src/main.rs`) | **587.70 ms** | **2.49x** |
| **Go** | `encoding/json/v2` | `map[string]any` with `clear(m)` reuse (`main_test.go`) | **2,364.44 ms** | **10.02x** |
| **Go** | `encoding/json/v2` | `map[string]any` fresh allocations (`main_test.go`) | **2,570.84 ms** | **10.89x** |
| **Go** | `encoding/json` v1 | `map[string]any` with `clear(m)` reuse (`main_test.go`) | **3,710.16 ms** | **15.72x** |
| **Go** | `encoding/json` v1 | `map[string]any` baseline (`main.go`) | **4,005.47 ms** | **16.97x slower** |

### Why Go Dynamic Parsing is ~7x Slower than Rust (~17x Slower than Goose)
- **Memory allocations & GC pressure:** Unmarshaling an untyped payload into `map[string]any` allocates nested map buckets, slice headers for arrays (`"tags"`), and boxes primitive values (`float64`, `string`) into interface values (`any`). This performs **24 heap allocations (976 B/op)** per parse—totaling **24 million heap allocations** (~976 MB memory allocated) across 1,000,000 iterations, keeping the Go GC under continuous load.
- **Reflection:** Standard `encoding/json` v1 relies heavily on runtime reflection to decode into arbitrary interface types. `encoding/json/v2` optimizes reflection and reduces allocations from 24 to 16 allocs/op, improving time to **2,571 ms** (~1.5x speedup over v1).
- **Rust & Goose design:** Rust's `serde_json::Value` is a compact enum parsed with no garbage collector overhead. Goose's native `json` parser reuses internal node and decoding buffers (`doc`, `decoded`) across iterations into a flat arena—no per-node heap allocations, no GC—which is why the compiled `main.goose` (**236 ms**) edges out `serde_json::Value` (**588 ms**).

---

## 2. Streaming JSON Parsing (Token-by-Token / Visitor)

Streaming parsers do not construct an in-memory dynamic AST or DOM. Instead, they scan the byte stream token-by-token (or byte-by-byte), matching specific keys on the fly and skipping unneeded branches without allocating intermediate structures.

Comparing streaming parsers to dynamic AST parsers is fundamentally an apples-to-oranges comparison. As shown below, when Rust is also measured using streaming token parsing, Rust is **~3.1x faster than Go's standard streaming decoder**.

### Measured Results (1,000,000 iterations)

| Language | Approach | Implementation | Measured Time | Relative Speed |
|:---|:---|:---|:---|:---|
| **Goose** | `json::reader` Token Stream | `json::each_member` cursor over reused `scratch`, `goose build` (`streaming.goose`) | **186.00 ms** | **1.00x (fastest)** |
| **Rust** | Zero-Copy Token Scanner | `TokenScanner` byte tokenizer, AWS-Smithy style (`src/main.rs`) | **213.66 ms** | **1.15x** |
| **Rust** | `serde` Streaming Visitor | `Deserializer::deserialize_map` with `Visitor` (`src/main.rs`) | **218.25 ms** | **1.17x** |
| **Go** | `encoding/json/jsontext` | `jsontext.Decoder` with buffer reset (`main_test.go`) | **671.94 ms** | **3.61x** |
| **Go** | `encoding/json/jsontext` | `jsontext.Decoder` fresh without buffer reset (`main_test.go`) | **1,395.00 ms** | **7.50x** |
| **Go** | `encoding/json` v1 | `json.NewDecoder` token streaming (`main_test.go`) | **2,730.00 ms** | **14.68x slower** |

### Streaming Analysis & Production Ecosystems
- **Goose Streaming (~186 ms):** `streaming.goose` walks tokens with `json::reader` / `json::each_member`, materializing strings into one reused `scratch` buffer and never building a DOM. At **186 ms** it is **1.27x faster than Goose's own DOM parser** (236 ms), **1.15x faster than Rust's zero-copy scanner**, and **3.61x faster than Go's best streaming decoder**—handling ~5.4 million payloads per second per core. (Under the `goose run` interpreter the same loop takes ~927 ms, so always benchmark Goose with `goose build`.)
- **Rust Streaming (~214–218 ms):** By parsing tokens directly from bytes with zero heap allocation, Rust processes ~4.7 million JSON payloads per second per core.
  - **AWS Smithy (`aws-smithy-json`):** AWS utilizes this exact token-streaming model in the AWS SDK for Rust to parse wire protocols with zero allocations and minimal CPU overhead.
  - **SIMD Accelerators (`sonic-rs` / `simd-json`):** Libraries like `sonic-rs` take this further by utilizing AVX2/SSE/NEON vector instructions to scan and check tag existence at gigabytes per second without full deserialization.
- **Go Streaming (`jsontext.Decoder` ~672 ms):** Go 1.27's standard library `encoding/json/jsontext.Decoder` provides low-level token-streaming primitives. When resetting over a reused buffer (`dec.Reset(buf)`), it achieves 1 alloc/op (8 bytes), bringing Go from ~4.0s down to **672 ms**.

---

## 3. Reference: Static Schema (Typed Struct Deserialization)

When the schema is known at compile time, unmarshaling directly into a statically typed Go struct bypasses untyped map allocations:

| Language | Approach | Implementation | Measured Time |
|:---|:---|:---|:---:|
| **Go** | `encoding/json/v2` | `payloadStruct` unmarshal (`main_test.go`) | **966.23 ms** |
| **Go** | `encoding/json` v1 | `payloadStruct` unmarshal (`main_test.go`) | **1,228.42 ms** |

---

## 4. Go Micro-Benchmarks & Memory Allocation Profile

Measured via `go test -bench=. -benchmem -count=3 -run=^$` (median of 3 runs):

| Category | Benchmark Routine | Speed (`ns/op`) | Memory (`B/op`) | Allocs (`allocs/op`) |
|:---|:---|:---:|:---:|:---:|
| **Streaming** | `BenchmarkV2_JsontextDecoder_Reused` | **676.9 ns/op** | 8 B/op | 1 allocs/op |
| **Streaming** | `BenchmarkV2_JsontextDecoder_Fresh` | **1,395.0 ns/op** | 736 B/op | 17 allocs/op |
| **Streaming** | `BenchmarkV1_Decoder` | **2,730.0 ns/op** | 336 B/op | 22 allocs/op |
| **Static Struct** | `BenchmarkV2_Struct` | **953.1 ns/op** | 64 B/op | 1 allocs/op |
| **Static Struct** | `BenchmarkV1_Struct` | **1,177.0 ns/op** | 64 B/op | 1 allocs/op |
| **Dynamic DOM** | `BenchmarkV2_MapReused` | **2,386.0 ns/op** | 528 B/op | 14 allocs/op |
| **Dynamic DOM** | `BenchmarkV2_Map` | **2,560.0 ns/op** | 864 B/op | 16 allocs/op |
| **Dynamic DOM** | `BenchmarkV1_MapReused` | **3,687.0 ns/op** | 640 B/op | 22 allocs/op |
| **Dynamic DOM** | `BenchmarkV1_Map` (current `main.go`) | **3,874.0 ns/op** | 976 B/op | 24 allocs/op |

---

## How to Run

Using Just:

```bash
just all          # Run original Go, Rust, and Goose benchmarks
just go           # Run Go baseline (go run main.go)
just rust         # Run Rust benchmarks: dynamic, streaming visitor, and token scanner (cargo run --release)
just goose        # Run Goose benchmark interpreted (goose run main.goose)
just goose-native # Build optimized Goose binaries and run DOM + streaming (goose build)
just compare-go   # Run 1,000,000 iterations comparison across all Go routines
just bench-go     # Run Go micro-benchmarks with memory allocations
```

Or run Goose directly:

```bash
goose build main.goose && ./main         # DOM benchmark (native)
goose build streaming.goose && ./streaming # Streaming benchmark (native)
```
