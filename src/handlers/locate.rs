use worker::*;
use wilayah::{haversine_km, location_from_village, Village};

use crate::db::query_nearest_candidates;
use crate::models::LocateResponse;
use crate::utils::cors::with_cors_and_cache;
use crate::utils::error::error_response;
use crate::utils::params::parse_f64_param;

pub async fn handle_locate(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let url = req.url()?;
    let lat = match parse_f64_param(&url, "lat") {
        Some(v) if (-90.0..=90.0).contains(&v) => v,
        _ => return error_response("Invalid or missing 'lat' parameter", 400),
    };
    let lon = match parse_f64_param(&url, "lon") {
        Some(v) if (-180.0..=180.0).contains(&v) => v,
        _ => return error_response("Invalid or missing 'lon' parameter", 400),
    };

    let rows = query_nearest_candidates(&ctx.env, lat, lon).await?;

    if rows.is_empty() {
        return with_cors_and_cache(
            Response::from_json(&LocateResponse { result: None }),
            30,
        );
    }

    let mut candidates: Vec<(crate::models::VillageRow, f64)> = rows
        .into_iter()
        .map(|r| {
            let d = haversine_km(lat, lon, r.lat, r.lon);
            (r, d)
        })
        .collect();
    candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    if let Some((village_row, dist_km)) = candidates.into_iter().next() {
        let village = Village::from(&village_row);
        if let Some(loc) = location_from_village(&village, dist_km) {
            return with_cors_and_cache(
                Response::from_json(&LocateResponse { result: Some(loc) }),
                30,
            );
        }
    }

    with_cors_and_cache(
        Response::from_json(&LocateResponse { result: None }),
        30,
    )
}
