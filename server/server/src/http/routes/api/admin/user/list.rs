use common::curia;
use std::collections::HashSet;

use common::Game;
use common::response::PaginatedResponse;
use common::response::admin::AdminUserRow;
use rocket::{State, http::Status, serde::json::Json};
use rocket_okapi::openapi;

use crate::config::Permissions;
use crate::http::guards::AdminGuard;
use crate::http::pool::Db;
use crate::services::AdminUserService;
use crate::stream::quic::CacheManager;

/// The server roster: every registered player, paginated and searchable.
///
/// Registration is what admits a login, so this is the whitelist as an operator reads it.
/// `connected` is live state; everything else is the player row.
#[openapi(tag = "Admin")]
#[get("/user?<page>&<page_size>&<search>&<game>")]
pub async fn list_users(
    _admin: AdminGuard,
    db: Db<'_>,
    perm_config: &State<Permissions>,
    cache_manager: &State<CacheManager>,
    page: Option<u32>,
    page_size: Option<u32>,
    search: Option<String>,
    game: Option<Game>,
) -> Result<Json<PaginatedResponse<AdminUserRow>>, Status> {
    let conn = db.into_inner();

    // One snapshot for the whole page. A per-row registry call would measure each row at
    // a different instant, so two rows could disagree about the same reconnect.
    let live: HashSet<String> = match cache_manager.get_connection_registry() {
        Some(registry) => registry.on_voice_identities(),
        None => HashSet::new(),
    };

    let page_size = AdminUserService::clamp_page_size(page_size);

    AdminUserService::list(
        conn,
        &perm_config.defaults,
        &live,
        page.unwrap_or(0),
        page_size,
        search,
        game,
    )
    .await
    .map(Json)
    .map_err(|e| {
        curia::error!("list_users: db error: {}", e);
        Status::InternalServerError
    })
}
