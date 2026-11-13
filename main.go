package main

import (
	"fmt"
	"time"

	jsonv2 "encoding/json/v2"
)

type result struct {
	svc     string
	ver     int
	latency float64
	owner   bool
}

func parse(data []byte) result {
	res := result{"", 0, 0, false}
	var m map[string]any
	jsonv2.Unmarshal(data, &m)

	// 1. service
	if svc, ok := m["service"].(string); ok {
		res.svc = svc
	}

	// 2. version (encoding/json/v2 decodes integers as int64)
	if ver, ok := m["version"].(int64); ok {
		res.ver = int(ver)
	} else if ver, ok := m["version"].(float64); ok {
		res.ver = int(ver)
	}

	// 3. latency_ms
	if metrics, ok := m["metrics"].(map[string]any); ok {
		if lat, ok := metrics["latency_ms"].(float64); ok {
			res.latency = lat
		}
	}

	// 4. owner
	_, ok := m["owner"]
	res.owner = ok

	return res
}

func main() {
	data := []byte(`{
	"service": "auth-api",
	"version": 2,
	"metrics": {"latency_ms": 15.4},
	"tags": ["core", "v2"]
    }`)

	start := time.Now()
	var r result
	for range 1_000_000 {
		r = parse(data)
	}
	fmt.Printf("Took: %v result: %v\n", time.Since(start), r)
}
