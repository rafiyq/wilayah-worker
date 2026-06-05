use worker::*;

use crate::db::update_villages;
use crate::models::{UpdatePayload, UpdateResponse};
use crate::utils::auth::check_auth;
use crate::utils::cors::with_cors;
use crate::utils::error::error_response;

pub async fn handle_update(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    if let Err(_) = check_auth(&req, &ctx.env) {
        return error_response("Unauthorized", 401);
    }
    let payload: UpdatePayload = match req.json().await {
        Ok(p) => p,
        Err(_) => return error_response("Invalid JSON body", 400),
    };
    if payload.villages.is_empty() {
        return error_response("No villages provided", 400);
    }
    if payload.villages.len() > 500 {
        return error_response("Max 500 villages per request", 400);
    }

    let count = payload.villages.len();
    update_villages(&ctx.env, &payload.villages).await?;

    with_cors(Response::from_json(&UpdateResponse { upserted: count }))
}
