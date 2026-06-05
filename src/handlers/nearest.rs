use worker::*;
use wilayah::{haversine_km, Village};

use crate::db::query_nearest_candidates;
use crate::models::NearestResponse;
use crate::utils::cors::with_cors_and_cache;
use crate::utils::error::error_response;
use crate::utils::params::{parse_f64_param, parse_usize_param};

pub async fn handle_nearest(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let url = req.url()?;
    let lat = match parse_f64_param(&url, "lat") {
        Some(v) if (-90.0..=90.0).contains(&v) => v,
        _ => return error_response("Invalid or missing 'lat' parameter", 400),
    };
    let lon = match parse_f64_param(&url, "lon") {
        Some(v) if (-180.0..=180.0).contains(&v) => v,
        _ => return error_response("Invalid or missing 'lon' parameter", 400),
    };
    let limit = parse_usize_param(&url, "limit", 5).clamp(1, 20);

    let rows = query_nearest_candidates(&ctx.env, lat, lon).await?;

    let candidates: Vec<Village> = rows
        .iter()
        .map(|r| {
            let mut v = Village::from(r);
            v.dist_km = Some(haversine_km(lat, lon, r.lat, r.lon));
            v
        })
        .collect();

    if candidates.is_empty() {
        return with_cors_and_cache(Response::from_json(&NearestResponse { results: vec![] }), 30);
    }

    let mut sorted = candidates;
    sorted.sort_by(|a, b| a.dist_km.unwrap().partial_cmp(&b.dist_km.unwrap()).unwrap());
    sorted.truncate(limit);

    with_cors_and_cache(Response::from_json(&NearestResponse { results: sorted }), 30)
}
