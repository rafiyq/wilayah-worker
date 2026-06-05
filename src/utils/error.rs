use crate::models::responses::ErrorResponse;
use crate::utils::cors::with_cors;
use worker::*;

pub fn error_response(msg: &str, status: u16) -> Result<Response> {
    let body = serde_json::to_string(&ErrorResponse {
        error: msg.to_string(),
    })
    .map_err(|e| Error::from(format!("serialize error: {e}")))?;
    with_cors(Response::error(&body, status))
}
