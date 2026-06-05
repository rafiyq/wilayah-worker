use worker::*;
use wilayah::{haversine_km, Village};

use crate::db::{get_village_count, query_nearest_candidates};
use crate::models::IndexResponse;
use crate::utils::constants::INDEX_HTML;
use crate::utils::cors::{with_cors, with_cors_and_cache};
use crate::utils::params::parse_f64_param;

pub async fn handle_index(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let is_html = req.headers().get("Accept")
        .ok().flatten()
        .map(|v| v.contains("text/html"))
        .unwrap_or(false);

    if is_html {
        let mut resp = Response::ok(INDEX_HTML)?;
        resp.headers_mut().set("Content-Type", "text/html; charset=utf-8")?;
        return with_cors(Ok(resp));
    }

    let count = get_village_count(&ctx.env).await?;

    let url = req.url()?;
    let mut source: Option<String> = None;
    let mut lat: Option<f64> = None;
    let mut lon: Option<f64> = None;

    // 1. Try query params first (GPS from device)
    if let (Some(lat_val), Some(lon_val)) = (parse_f64_param(&url, "lat"), parse_f64_param(&url, "lon")) {
        if (-90.0..=90.0).contains(&lat_val) && (-180.0..=180.0).contains(&lon_val) {
            lat = Some(lat_val);
            lon = Some(lon_val);
            source = Some("gps".to_string());
        }
    }

    // 2. Fall back to Cloudflare IP geolocation headers
    if lat.is_none() {
        let cf_lat_str = req.headers().get("CF-IPLatitude").ok().flatten();
        let cf_lon_str = req.headers().get("CF-IPLongitude").ok().flatten();
        if let (Some(lat_val), Some(lon_val)) = (
            cf_lat_str.and_then(|s| s.parse().ok()),
            cf_lon_str.and_then(|s| s.parse().ok()),
        ) {
            lat = Some(lat_val);
            lon = Some(lon_val);
            source = Some("ip".to_string());
        }
    }

    let mut nearest: Option<Vec<Village>> = None;

    if let (Some(lat_val), Some(lon_val)) = (lat, lon) {
        let limit = 5;
        let rows = query_nearest_candidates(&ctx.env, lat_val, lon_val).await?;
        if !rows.is_empty() {
            let mut candidates: Vec<Village> = rows
                .iter()
                .map(|r| {
                    let mut v = Village::from(r);
                    v.dist_km = Some(haversine_km(lat_val, lon_val, r.lat, r.lon));
                    v
                })
                .collect();
            candidates.sort_by(|a, b| {
                a.dist_km
                    .unwrap()
                    .partial_cmp(&b.dist_km.unwrap())
                    .unwrap()
            });
            candidates.truncate(limit);
            nearest = Some(candidates);
        }
    }

    with_cors_and_cache(
        Response::from_json(&IndexResponse {
            name: "wilayah".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            village_count: count,
            source,
            lat,
            lon,
            nearest,
        }),
        30,
    )
}
