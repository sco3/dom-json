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

*All timings are medians of repeated runs measured directly on an AMD Ryzen 5 4500U (Linux x86_64, Go 1.27.2, Rust 1.85, Goose 0.4). Goose numbers are from optimized native binaries built with `goose build` (`main-goose`, `streaming-goose`):*

| Language | Approach | Implementation | Measured Time | Relative Speed |
|:---|:---|:---|:---|:---|
| **Goose** | `json::Node` | Native AST DOM with buffer reuse, `goose build` (`main.goose`) | **236.00 ms** | **1.00x (fastest)** |
| **Rust** | `serde_json::Value` | Dynamic DOM enum tree (`src/main.rs`) | **587.70 ms** | **2.49x** |
| **Go** | `encoding/json/v2` | `map[string]any` with `clear(m)` reuse (`main_test.go`) | **2,364.44 ms** | **10.02x** |

### Why Go Dynamic Parsing is ~4x Slower than Rust (~10x Slower than Goose)
- **Memory allocations & GC pressure:** Unmarshaling an untyped payload into `map[string]any` allocates nested map buckets, slice headers for arrays (`"tags"`), and boxes primitive values (`float64`, `string`) into interface values (`any`). This performs **14 heap allocations (528 B/op)** per parse with buffer reuse—totaling **14 million heap allocations** (~528 MB memory allocated) across 1,000,000 iterations, keeping the Go GC under continuous load.
- **Reflection:** Decoding into untyped `any` values requires runtime reflection to resolve types on every field. Even the optimized `encoding/json/v2` still pays this per-value reflection cost, keeping dynamic map parsing an order of magnitude behind typed struct parsing and compiled parsers.
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

### Streaming Analysis & Production Ecosystems
- **Goose Streaming (~186 ms):** `streaming.goose` walks tokens with `json::reader` / `json::each_member`, materializing strings into one reused `scratch` buffer and never building a DOM. At **186 ms** it is **1.27x faster than Goose's own DOM parser** (236 ms), **1.15x faster than Rust's zero-copy scanner**, and **3.61x faster than Go's best streaming decoder**—handling ~5.4 million payloads per second per core.
- **Rust Streaming (~214–218 ms):** By parsing tokens directly from bytes with zero heap allocation, Rust processes ~4.7 million JSON payloads per second per core.
  - **AWS Smithy (`aws-smithy-json`):** AWS utilizes this exact token-streaming model in the AWS SDK for Rust to parse wire protocols with zero allocations and minimal CPU overhead.
  - **SIMD Accelerators (`sonic-rs` / `simd-json`):** Libraries like `sonic-rs` take this further by utilizing AVX2/SSE/NEON vector instructions to scan and check tag existence at gigabytes per second without full deserialization.
- **Go Streaming (`jsontext.Decoder` ~672 ms):** Go 1.27's standard library `encoding/json/jsontext.Decoder` provides low-level token-streaming primitives. When resetting over a reused buffer (`dec.Reset(buf)`), it achieves 1 alloc/op (8 bytes), bringing Go dynamic parsing from ~2.4s down to **672 ms**.

---

## 3. Reference: Static Schema (Typed Struct Deserialization)

When the schema is known at compile time, unmarshaling directly into a statically typed Go struct bypasses untyped map allocations:

| Language | Approach | Implementation | Measured Time |
|:---|:---|:---|:---:|
| **Go** | `encoding/json/v2` | `payloadStruct` unmarshal (`main_test.go`) | **966.23 ms** |

---

## 4. Go Micro-Benchmarks & Memory Allocation Profile

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
