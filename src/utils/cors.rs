use worker::*;

pub fn with_cors(response: Result<Response>) -> Result<Response> {
    response.map(|r| {
        let h = Headers::new();
        h.set("Access-Control-Allow-Origin", "*").unwrap();
        h.set("Access-Control-Allow-Methods", "GET, PUT, OPTIONS")
            .unwrap();
        h.set("Access-Control-Allow-Headers", "*").unwrap();
        r.with_headers(h)
    })
}

pub fn with_cors_and_cache(response: Result<Response>, max_age: u32) -> Result<Response> {
    response.map(|r| {
        let h = Headers::new();
        h.set("Access-Control-Allow-Origin", "*").unwrap();
        h.set("Access-Control-Allow-Methods", "GET, PUT, OPTIONS")
            .unwrap();
        h.set("Access-Control-Allow-Headers", "*").unwrap();
        h.set("Cache-Control", &format!("public, max-age={}", max_age))
            .unwrap();
        r.with_headers(h)
    })
}
