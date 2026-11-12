package main

import (
	"encoding/json"
	"fmt"
	"time"
)

type result struct {
	svc     string
	ver     int
	latency float64
	owner   bool
}

func parse(data []byte) result {
	result := result{"", 0, 0, false}
	var m map[string]any
	json.Unmarshal(data, &m)

	// 1. service
	if svc, ok := m["service"].(string); ok {
		//fmt.Println("Service:", svc)
		result.svc = svc
	}

	// 2. version (в map[string]any все числа - float64)
	if ver, ok := m["version"].(float64); ok {
		//fmt.Println("Version:", int(ver))
		result.ver = int(ver)
	}

	// 3. latency_ms (двойная типизация)
	if metrics, ok := m["metrics"].(map[string]any); ok {
		if lat, ok := metrics["latency_ms"].(float64); ok {
			//fmt.Println("Latency:", lat)
			result.latency = lat

		}
	}

	// 4. owner
	_, ok := m["owner"]
	result.owner = ok

	return result

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
