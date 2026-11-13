import json;
import std;

struct Result {
    String svc;
    i64 ver;
    double latency;
    bool owner;
}

fn void main()
{
    String payload = `{
        "service": "auth-api",
        "version": 2,
        "metrics": {
            "latency_ms": 15.4
        },
        "tags": ["core", "v2"]
    }`;

    // Scratch buffer for string reads
    char[256] scratch;
    char[] scratch_slice = scratch[..];

    Result res;
    res.owner = false;

    i64 start = std::time::now();

    for (int i = 0; i < 1_000_000; i++)
    {
        JsonReader r = json::reader(payload);
        json::expect(r, json::Token.OBJECT_START);

        while (json::has_next(&r))
        {
            String key = json::next_key(&r);
            switch (key)
            {
                case "service":
                    res.svc = json::read_string(&r, scratch_slice);
                case "version":
                    res.ver = json::read_int(&r);
                case "metrics":
                    json::expect(&r, json::Token.OBJECT_START);
                    while (json::has_next(&r))
                    {
                        String mkey = json::next_key(&r);
                        if (mkey == "latency_ms")
                            res.latency = json::read_number(&r);
                        else
                            json::skip(&r);
                    }
                    json::expect(&r, json::Token.OBJECT_END);
                case "owner":
                    json::skip(&r);
                    res.owner = true;
                default:
                    json::skip(&r);
            }
        }
        json::expect(&r, json::Token.OBJECT_END);
    }

    i64 elapsed_ms = (std::time::now() - start) / 1_000_000;
    io::printfn("Took: %d ms", elapsed_ms);
}