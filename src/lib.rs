use worker::*;

mod db;
mod handlers;
mod models;
mod utils;

use handlers::code::handle_code;
use handlers::cors::handle_cors;
use handlers::index::handle_index;
use handlers::locate::handle_locate;
use handlers::nearest::handle_nearest;
use handlers::search::handle_search;
use handlers::update::handle_update;
use handlers::update_meta::handle_update_meta;

#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let router = Router::new();

    router
        .get_async("/", handle_index)
        .get_async("/nearest", handle_nearest)
        .get_async("/search", handle_search)
        .get_async("/code", handle_code)
        .get_async("/locate", handle_locate)
        .put_async("/update", handle_update)
        .put_async("/update/meta", handle_update_meta)
        .options_async("/*catchall", handle_cors)
        .run(req, env)
        .await
}
