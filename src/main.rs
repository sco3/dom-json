use serde::de::{self, DeserializeSeed, Deserializer, MapAccess, Visitor};
use serde_json::Value;
use std::time::Instant;

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct Result {
    svc: String,
    ver: i64,
    latency: f64,
    owner: bool,
}

// ---------------------------------------------------------------------------
// 1. Dynamic DOM / AST Parsing (serde_json::Value) - Baseline
// ---------------------------------------------------------------------------
fn parse_dynamic_value(data: &str) -> Result {
    let v: Value = serde_json::from_str(data).unwrap();
    Result {
        svc: v["service"].as_str().unwrap_or("").to_string(),
        ver: v["version"].as_i64().unwrap_or(0),
        latency: v["metrics"]["latency_ms"].as_f64().unwrap_or(0.0),
        owner: !v["owner"].is_null(),
    }
}

// ---------------------------------------------------------------------------
// 2. Streaming Visitor (serde streaming deserializer, zero-alloc)
// ---------------------------------------------------------------------------
struct ResultVisitor;
impl<'de> Visitor<'de> for ResultVisitor {
    type Value = Result;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a JSON object")
    }

    fn visit_map<M>(self, mut access: M) -> std::result::Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let mut svc = String::new();
        let mut ver = 0i64;
        let mut latency = 0.0f64;
        let mut owner = false;

        while let Some(key) = access.next_key::<&str>()? {
            match key {
                "service" => {
                    svc = access.next_value::<&str>()?.to_string();
                }
                "version" => {
                    ver = access.next_value::<i64>()?;
                }
                "metrics" => {
                    struct MetricsVisitor;
                    impl<'de> Visitor<'de> for MetricsVisitor {
                        type Value = f64;
                        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                            formatter.write_str("metrics object")
                        }
                        fn visit_map<M2>(self, mut maccess: M2) -> std::result::Result<Self::Value, M2::Error>
                        where
                            M2: MapAccess<'de>,
                        {
                            let mut lat = 0.0;
                            while let Some(mkey) = maccess.next_key::<&str>()? {
                                if mkey == "latency_ms" {
                                    lat = maccess.next_value::<f64>()?;
                                } else {
                                    let _ = maccess.next_value::<de::IgnoredAny>()?;
                                }
                            }
                            Ok(lat)
                        }
                    }
                    struct MetricsSeed;
                    impl<'de> DeserializeSeed<'de> for MetricsSeed {
                        type Value = f64;
                        fn deserialize<D>(self, deserializer: D) -> std::result::Result<Self::Value, D::Error>
                        where
                            D: Deserializer<'de>,
                        {
                            deserializer.deserialize_map(MetricsVisitor)
                        }
                    }
                    latency = access.next_value_seed(MetricsSeed)?;
                }
                "owner" => {
                    owner = true;
                    let _ = access.next_value::<de::IgnoredAny>()?;
                }
                _ => {
                    let _ = access.next_value::<de::IgnoredAny>()?;
                }
            }
        }

        Ok(Result { svc, ver, latency, owner })
    }
}

fn parse_streaming_serde(data: &str) -> Result {
    let mut deserializer = serde_json::Deserializer::from_str(data);
    deserializer.deserialize_map(ResultVisitor).unwrap()
}

// ---------------------------------------------------------------------------
// 3. Fast Zero-Copy Token Stream (aws-smithy / sonic style token scanner)
// ---------------------------------------------------------------------------
struct TokenScanner<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> TokenScanner<'a> {
    #[inline(always)]
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    #[inline(always)]
    fn skip_whitespace(&mut self) {
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            if b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    #[inline(always)]
    fn next_string(&mut self) -> Option<&'a str> {
        self.skip_whitespace();
        if self.pos >= self.bytes.len() || self.bytes[self.pos] != b'"' {
            return None;
        }
        self.pos += 1;
        let start = self.pos;
        while self.pos < self.bytes.len() && self.bytes[self.pos] != b'"' {
            if self.bytes[self.pos] == b'\\' {
                self.pos += 2;
            } else {
                self.pos += 1;
            }
        }
        let end = self.pos;
        if self.pos < self.bytes.len() {
            self.pos += 1; // skip closing "
        }
        std::str::from_utf8(&self.bytes[start..end]).ok()
    }

    #[inline(always)]
    fn skip_colon(&mut self) {
        self.skip_whitespace();
        if self.pos < self.bytes.len() && self.bytes[self.pos] == b':' {
            self.pos += 1;
        }
    }

    #[inline(always)]
    fn parse_i64(&mut self) -> i64 {
        self.skip_whitespace();
        let start = self.pos;
        if self.pos < self.bytes.len() && (self.bytes[self.pos] == b'-' || self.bytes[self.pos] == b'+') {
            self.pos += 1;
        }
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
        std::str::from_utf8(&self.bytes[start..self.pos]).unwrap_or("0").parse().unwrap_or(0)
    }

    #[inline(always)]
    fn parse_f64(&mut self) -> f64 {
        self.skip_whitespace();
        let start = self.pos;
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            if b.is_ascii_digit() || b == b'.' || b == b'-' || b == b'+' || b == b'e' || b == b'E' {
                self.pos += 1;
            } else {
                break;
            }
        }
        std::str::from_utf8(&self.bytes[start..self.pos]).unwrap_or("0").parse().unwrap_or(0.0)
    }

    #[inline(always)]
    fn skip_value(&mut self) {
        self.skip_whitespace();
        if self.pos >= self.bytes.len() {
            return;
        }
        match self.bytes[self.pos] {
            b'"' => {
                self.next_string();
            }
            b'{' | b'[' => {
                let open = self.bytes[self.pos];
                let close = if open == b'{' { b'}' } else { b']' };
                self.pos += 1;
                let mut depth = 1;
                while self.pos < self.bytes.len() && depth > 0 {
                    let b = self.bytes[self.pos];
                    if b == b'"' {
                        self.next_string();
                    } else {
                        if b == open {
                            depth += 1;
                        } else if b == close {
                            depth -= 1;
                        }
                        self.pos += 1;
                    }
                }
            }
            _ => {
                while self.pos < self.bytes.len() {
                    let b = self.bytes[self.pos];
                    if b == b',' || b == b'}' || b == b']' || b == b' ' || b == b'\n' || b == b'\r' || b == b'\t' {
                        break;
                    }
                    self.pos += 1;
                }
            }
        }
    }
}

fn parse_streaming_token_scanner(data: &[u8]) -> Result {
    let mut s = TokenScanner::new(data);
    s.skip_whitespace();
    if s.pos < s.bytes.len() && s.bytes[s.pos] == b'{' {
        s.pos += 1;
    }

    let mut r = Result {
        svc: String::new(),
        ver: 0,
        latency: 0.0,
        owner: false,
    };

    while s.pos < s.bytes.len() {
        s.skip_whitespace();
        if s.pos >= s.bytes.len() || s.bytes[s.pos] == b'}' {
            break;
        }
        if s.bytes[s.pos] == b',' {
            s.pos += 1;
            s.skip_whitespace();
        }
        if let Some(key) = s.next_string() {
            s.skip_colon();
            match key {
                "service" => {
                    if let Some(val) = s.next_string() {
                        r.svc = val.to_string();
                    }
                }
                "version" => {
                    r.ver = s.parse_i64();
                }
                "metrics" => {
                    s.skip_whitespace();
                    if s.pos < s.bytes.len() && s.bytes[s.pos] == b'{' {
                        s.pos += 1;
                        while s.pos < s.bytes.len() {
                            s.skip_whitespace();
                            if s.pos >= s.bytes.len() || s.bytes[s.pos] == b'}' {
                                break;
                            }
                            if s.bytes[s.pos] == b',' {
                                s.pos += 1;
                            }
                            if let Some(mkey) = s.next_string() {
                                s.skip_colon();
                                if mkey == "latency_ms" {
                                    r.latency = s.parse_f64();
                                } else {
                                    s.skip_value();
                                }
                            }
                        }
                        if s.pos < s.bytes.len() && s.bytes[s.pos] == b'}' {
                            s.pos += 1;
                        }
                    } else {
                        s.skip_value();
                    }
                }
                "owner" => {
                    r.owner = true;
                    s.skip_value();
                }
                _ => {
                    s.skip_value();
                }
            }
        }
    }
    r
}

fn main() {
    let data = r#"{ "service": "auth-api", "version": 2, "metrics": {"latency_ms": 15.4}, "tags": ["core", "v2"] }"#;
    const N: usize = 1_000_000;

    // 1. Dynamic DOM (serde_json::Value)
    {
        let start = Instant::now();
        let mut r = Result { svc: String::new(), ver: 0, latency: 0.0, owner: false };
        for _ in 0..N {
            r = parse_dynamic_value(data);
        }
        println!("Rust dynamic (serde_json::Value):    {:?} | result: {:?}", start.elapsed(), r);
    }

    // 2. Streaming Visitor (serde streaming)
    {
        let start = Instant::now();
        let mut r = Result { svc: String::new(), ver: 0, latency: 0.0, owner: false };
        for _ in 0..N {
            r = parse_streaming_serde(data);
        }
        println!("Rust streaming (serde Visitor):      {:?} | result: {:?}", start.elapsed(), r);
    }

    // 3. Fast Zero-Copy Token Scanner (aws-smithy / sonic style token stream)
    {
        let bytes = data.as_bytes();
        let start = Instant::now();
        let mut r = Result { svc: String::new(), ver: 0, latency: 0.0, owner: false };
        for _ in 0..N {
            r = parse_streaming_token_scanner(bytes);
        }
        println!("Rust streaming (token scanner):      {:?} | result: {:?}", start.elapsed(), r);
    }
}
