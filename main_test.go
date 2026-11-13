package main

import (
	"bytes"
	"encoding/json/jsontext"
	jsonv2 "encoding/json/v2"
	"fmt"
	"testing"
	"time"
)

var benchmarkData = []byte(`{
	"service": "auth-api",
	"version": 2,
	"metrics": {"latency_ms": 15.4},
	"tags": ["core", "v2"]
}`)

// Typed struct representation for struct benchmarks
type payloadStruct struct {
	Service string `json:"service"`
	Version int    `json:"version"`
	Metrics struct {
		Latency float64 `json:"latency_ms"`
	} `json:"metrics"`
	Owner jsontext.Value `json:"owner"`
}

// ---------------------------------------------------------------------------
// 1. V2 Map (encoding/json/v2 map[string]any)
// ---------------------------------------------------------------------------
func parseV2Map(data []byte) result {
	res := result{"", 0, 0, false}
	var m map[string]any
	jsonv2.Unmarshal(data, &m)

	if svc, ok := m["service"].(string); ok {
		res.svc = svc
	}
	if ver, ok := m["version"].(int64); ok {
		res.ver = int(ver)
	} else if ver, ok := m["version"].(float64); ok {
		res.ver = int(ver)
	}
	if metrics, ok := m["metrics"].(map[string]any); ok {
		if lat, ok := metrics["latency_ms"].(float64); ok {
			res.latency = lat
		}
	}
	_, ok := m["owner"]
	res.owner = ok
	return res
}

// ---------------------------------------------------------------------------
// 2. V2 Map with reuse (clear(m))
// ---------------------------------------------------------------------------
func parseV2MapReused(data []byte, m map[string]any) result {
	clear(m)
	jsonv2.Unmarshal(data, &m)

	res := result{"", 0, 0, false}
	if svc, ok := m["service"].(string); ok {
		res.svc = svc
	}
	if ver, ok := m["version"].(int64); ok {
		res.ver = int(ver)
	} else if ver, ok := m["version"].(float64); ok {
		res.ver = int(ver)
	}
	if metrics, ok := m["metrics"].(map[string]any); ok {
		if lat, ok := metrics["latency_ms"].(float64); ok {
			res.latency = lat
		}
	}
	_, ok := m["owner"]
	res.owner = ok
	return res
}

// ---------------------------------------------------------------------------
// 3. V2 jsontext.Decoder (Streaming token decoder with buffer reuse, like Goose)
// ---------------------------------------------------------------------------
func parseJsontextStream(dec *jsontext.Decoder) result {
	var r result
	tok, err := dec.ReadToken()
	if err != nil || tok.Kind() != '{' {
		return r
	}

	for {
		tok, err := dec.ReadToken()
		if err != nil || tok.Kind() == '}' {
			break
		}
		key := tok.String()
		switch key {
		case "service":
			val, _ := dec.ReadToken()
			r.svc = val.String()
		case "version":
			val, _ := dec.ReadToken()
			v, _ := val.Int()
			r.ver = int(v)
		case "metrics":
			val, _ := dec.ReadToken()
			if val.Kind() == '{' {
				for {
					mtok, err := dec.ReadToken()
					if err != nil || mtok.Kind() == '}' {
						break
					}
					if mtok.String() == "latency_ms" {
						lval, _ := dec.ReadToken()
						f, _ := lval.Float()
						r.latency = f
					} else {
						dec.SkipValue()
					}
				}
			} else {
				dec.SkipValue()
			}
		case "owner":
			r.owner = true
			dec.SkipValue()
		default:
			dec.SkipValue()
		}
	}
	return r
}

// ---------------------------------------------------------------------------
// 4. V2 Struct Unmarshal
// ---------------------------------------------------------------------------
func parseV2Struct(data []byte) result {
	var p payloadStruct
	jsonv2.Unmarshal(data, &p)
	return result{
		svc:     p.Service,
		ver:     p.Version,
		latency: p.Metrics.Latency,
		owner:   p.Owner != nil,
	}
}

// ===========================================================================
// Micro-Benchmarks (go test -bench=. -benchmem)
// ===========================================================================

func BenchmarkV2_Map(b *testing.B) {
	var r result
	for i := 0; i < b.N; i++ {
		r = parseV2Map(benchmarkData)
	}
	_ = r
}

func BenchmarkV2_MapReused(b *testing.B) {
	m := make(map[string]any, 8)
	var r result
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		r = parseV2MapReused(benchmarkData, m)
	}
	_ = r
}

func BenchmarkV2_JsontextDecoder_Reused(b *testing.B) {
	buf := bytes.NewBuffer(benchmarkData)
	dec := jsontext.NewDecoder(buf)
	var r result
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		buf.Reset()
		buf.Write(benchmarkData)
		dec.Reset(buf)
		r = parseJsontextStream(dec)
	}
	_ = r
}

func BenchmarkV2_JsontextDecoder_Fresh(b *testing.B) {
	var r result
	for i := 0; i < b.N; i++ {
		buf := bytes.NewBuffer(benchmarkData)
		dec := jsontext.NewDecoder(buf)
		r = parseJsontextStream(dec)
	}
	_ = r
}

func BenchmarkV2_Struct(b *testing.B) {
	var r result
	for i := 0; i < b.N; i++ {
		r = parseV2Struct(benchmarkData)
	}
	_ = r
}

// ===========================================================================
// 1,000,000 Iterations Test Comparison (matches main.go loop)
// Run with: go test -v -run TestCompareAll1M
// ===========================================================================

func TestCompareAll1M(t *testing.T) {
	const N = 1_000_000

	type testCase struct {
		name string
		fn   func() result
	}

	// Setup reused decoders/maps for those methods
	mReusedV2 := make(map[string]any, 8)

	buf := bytes.NewBuffer(benchmarkData)
	dec := jsontext.NewDecoder(buf)

	dynamicCases := []testCase{
		{
			name: "Go v2 map[string]any",
			fn:   func() result { return parseV2Map(benchmarkData) },
		},
		{
			name: "Go v2 map[string]any with clear(m) reuse",
			fn:   func() result { return parseV2MapReused(benchmarkData, mReusedV2) },
		},
	}

	streamingCases := []testCase{
		{
			name: "Go v2 jsontext.Decoder (buffer reuse)",
			fn: func() result {
				buf.Reset()
				buf.Write(benchmarkData)
				dec.Reset(buf)
				return parseJsontextStream(dec)
			},
		},
	}

	staticCases := []testCase{
		{
			name: "Go v2 typed struct",
			fn:   func() result { return parseV2Struct(benchmarkData) },
		},
	}

	fmt.Println("\n=== 1. Dynamic (Schemaless) JSON Parsing (1,000,000 iters) ===")
	for _, tc := range dynamicCases {
		start := time.Now()
		var r result
		for range N {
			r = tc.fn()
		}
		elapsed := time.Since(start)
		fmt.Printf("%-50s: %12v | result: %+v\n", tc.name, elapsed, r)
		if r.svc != "auth-api" || r.ver != 2 || r.latency != 15.4 || r.owner != false {
			t.Errorf("%s returned incorrect result: %+v", tc.name, r)
		}
	}

	fmt.Println("\n=== 2. Streaming JSON Parsing (Token-by-Token, 1,000,000 iters) ===")
	for _, tc := range streamingCases {
		start := time.Now()
		var r result
		for range N {
			r = tc.fn()
		}
		elapsed := time.Since(start)
		fmt.Printf("%-50s: %12v | result: %+v\n", tc.name, elapsed, r)
		if r.svc != "auth-api" || r.ver != 2 || r.latency != 15.4 || r.owner != false {
			t.Errorf("%s returned incorrect result: %+v", tc.name, r)
		}
	}

	fmt.Println("\n=== 3. Static Schema Struct Parsing (1,000,000 iters) ===")
	for _, tc := range staticCases {
		start := time.Now()
		var r result
		for range N {
			r = tc.fn()
		}
		elapsed := time.Since(start)
		fmt.Printf("%-50s: %12v | result: %+v\n", tc.name, elapsed, r)
		if r.svc != "auth-api" || r.ver != 2 || r.latency != 15.4 || r.owner != false {
			t.Errorf("%s returned incorrect result: %+v", tc.name, r)
		}
	}
	fmt.Println("=================================================================")
}
