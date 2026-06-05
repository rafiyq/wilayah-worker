use worker::*;

use crate::db::{get_village_by_code, get_village_count_by_prefix, get_villages_by_prefix};
use crate::models::{CodePrefixResponse, CodeResponse};
use crate::utils::cors::with_cors_and_cache;
use crate::utils::error::error_response;
use crate::utils::params::{parse_usize_param, query_param};

pub async fn handle_code(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let url = req.url()?;

    if let Some(q) = query_param(&url, "q") {
        let result = get_village_by_code(&ctx.env, &q).await?;
        return with_cors_and_cache(
            Response::from_json(&CodeResponse { result }),
            3600,
        );
    }

    if let Some(prefix) = query_param(&url, "prefix") {
        let limit = parse_usize_param(&url, "limit", 100).clamp(1, 1000);
        let offset = parse_usize_param(&url, "offset", 0);
        let pattern = format!("{prefix}%");

        let total = get_village_count_by_prefix(&ctx.env, &pattern).await?;
        let rows = get_villages_by_prefix(&ctx.env, &pattern, limit, offset).await?;
        let has_more = (offset + rows.len()) < total as usize;

        return with_cors_and_cache(
            Response::from_json(&CodePrefixResponse {
                results: rows,
                total,
                has_more,
            }),
            3600,
        );
    }

    error_response(
        "Provide either 'q' (exact code) or 'prefix' (code prefix)",
        400,
    )
}
