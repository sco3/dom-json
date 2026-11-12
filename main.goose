import json;
import std;

fn main() {
    let text = """
    {
        "service": "auth-api",
        "version": 2,
        "metrics": {
            "latency_ms": 15.4
        },
        "tags": ["core", "v2"]
    }
    """;

    var doc: json::Node[>..] = [];
    var decoded: u8[>..] = [];

    let root, err = json::parse(text, doc, decoded);
    guard root else { abort(err); }

    // 1. service
    let svc_node = json::member(root, "service");
    let svc, ok_svc = json::as_string(svc_node);
    if ok_svc { print("Service: ", svc); }

    // 2. version
    let ver_node = json::member(root, "version");
    let ver, ok_ver = json::as_i64(ver_node);
    if ok_ver { print("Version: ", ver); }

    // 3. latency_ms
    let metrics_node = json::member(root, "metrics");
    let lat_node = json::member(metrics_node, "latency_ms");
    let lat, ok_lat = json::as_f64(lat_node);
    if ok_lat { print("Latency: ", lat); }

    // 4. owner (null, no key)
    let owner_node = json::member(root, "owner");
    if !owner_node {
        print("Owner is missing (expected)");
    }
}