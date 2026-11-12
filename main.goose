import json;
import std;
import os;

struct Result {
svc: const u8[:],
ver: i64,
latency: f64,
owner: bool,
}

fn parse(text: u8[:], doc: json::Node[>..]&, decoded: u8[>..]&) -> Result {
let root, err = json::parse(text, doc, decoded);
guard root else { abort(err); }

var res = Result { svc: "", ver: 0, latency: 0.0, owner: false };

let svc_node = json::member(root, "service");
let svc, ok_svc = json::as_string(svc_node);
if ok_svc { res.svc = svc; }

let ver_node = json::member(root, "version");
let ver, ok_ver = json::as_i64(ver_node);
if ok_ver { res.ver = ver; }

let metrics_node = json::member(root, "metrics");
let lat_node = json::member(metrics_node, "latency_ms");
let lat, ok_lat = json::as_f64(lat_node);
if ok_lat { res.latency = lat; }

let owner_node = json::member(root, "owner");
if owner_node { res.owner = true; }

return res;
}

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
var r = Result { svc: "", ver: 0, latency: 0.0, owner: false };

var start = clock();

for i in 1000000 {
r = parse(text, doc, decoded);
}
var took: u32 = (1000*(clock() - start) ) as u32;
print ("Took: ",took, " ms ");

print("Result: ", r.svc, " ", r.ver, " ", r.latency, " ", r.owner);
}