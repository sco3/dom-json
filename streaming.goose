import json;
import std;
import os;

struct Result {
    svc:     const u8[:],
    ver:     i64,
    latency: f64,
    owner:   bool,
}

fn parse(text: u8[:], scratch: u8[>..]&) -> Result {
    var r = json::reader(text);
    var res = Result { svc: "", ver: 0, latency: 0.0, owner: false };

    json::each_member(r) { key =>
        if key == "service" { res.svc = json::read_string(r, scratch); }
        else if key == "version" { res.ver = json::read_int(r); }
        else if key == "metrics" {
            json::each_member(r) { field =>
                if field == "latency_ms" { res.latency = json::read_number(r); }
                else { json::skip(r); }
            };
        }
        else if key == "owner" { res.owner = true; json::skip(r); }
        else { json::skip(r); }
    };

    if !json::finish(r) { abort(json::message(r)); }
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

    // Scratch buffer for strings that hold escapes
    var scratch: u8[>..] = [];
    var res = Result { svc: "", ver: 0, latency: 0.0, owner: false };

    let start = clock();

    for i in 1000000 {
        res = parse(text, scratch);
    }

    var took: u32 = (1000 * (clock() - start)) as u32;
    print("Took: ", took, " ms ");

    print("Result: ", res.svc, " ", res.ver, " ", res.latency, " ", res.owner);
}
