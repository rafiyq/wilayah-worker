use worker::*;

use crate::db::search_villages;
use crate::models::SearchResponse;
use crate::utils::cors::with_cors_and_cache;
use crate::utils::error::error_response;
use crate::utils::params::{parse_usize_param, query_param};

pub async fn handle_search(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let url = req.url()?;
    let q = match query_param(&url, "q") {
        Some(v) if !v.is_empty() => v,
        _ => return error_response("Query parameter 'q' is required", 400),
    };
    let limit = parse_usize_param(&url, "limit", 10).clamp(1, 100);
    let pattern = format!("%{q}%");

    let rows = search_villages(&ctx.env, &pattern, limit).await?;
    with_cors_and_cache(Response::from_json(&SearchResponse { results: rows }), 60)
}
