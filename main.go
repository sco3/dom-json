package main

import (
	"encoding/json"
	"fmt"
)

func main() {
	data := []byte(`{
	"service": "auth-api",
	"version": 2,
	"metrics": {"latency_ms": 15.4},
	"tags": ["core", "v2"]
    }`)

	var m map[string]any
	json.Unmarshal(data, &m)

	// 1. service
	if svc, ok := m["service"].(string); ok {
		fmt.Println("Service:", svc)
	}

	// 2. version (в map[string]any все числа - float64)
	if ver, ok := m["version"].(float64); ok {
		fmt.Println("Version:", int(ver))
	}

	// 3. latency_ms (двойная типизация)
	if metrics, ok := m["metrics"].(map[string]any); ok {
		if lat, ok := metrics["latency_ms"].(float64); ok {
			fmt.Println("Latency:", lat)

		}
	}

	// 4. owner
	if _, exists := m["owner"]; !exists {
		fmt.Println("Owner is missing (expected)")
	}
}
