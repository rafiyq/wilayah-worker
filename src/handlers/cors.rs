use worker::*;

pub async fn handle_cors(_req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    let h = Headers::new();
    h.set("Access-Control-Allow-Origin", "*").unwrap();
    h.set("Access-Control-Allow-Methods", "GET, PUT, OPTIONS")
        .unwrap();
    h.set("Access-Control-Allow-Headers", "*").unwrap();
    Ok(Response::empty()?.with_headers(h))
}
