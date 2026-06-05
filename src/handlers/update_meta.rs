use worker::*;

use crate::db::update_meta;
use crate::models::{MetaUpdatePayload, UpdateResponse};
use crate::utils::auth::check_auth;
use crate::utils::cors::with_cors;
use crate::utils::error::error_response;

pub async fn handle_update_meta(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    if let Err(_) = check_auth(&req, &ctx.env) {
        return error_response("Unauthorized", 401);
    }
    let payload: MetaUpdatePayload = match req.json().await {
        Ok(p) => p,
        Err(_) => return error_response("Invalid JSON body", 400),
    };
    if payload.meta.is_empty() {
        return error_response("No metadata provided", 400);
    }

    let count = payload.meta.len();
    update_meta(&ctx.env, &payload.meta).await?;

    with_cors(Response::from_json(&UpdateResponse { upserted: count }))
}
