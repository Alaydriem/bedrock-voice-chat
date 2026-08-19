use std::collections::{HashMap, HashSet};

use common::Game;
use common::response::PaginatedResponse;
use common::response::admin::AdminUserRow;
use common::structs::permission::Permission;
use entity::{player, player_permission};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbErr, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
};

/// The server roster, as an operator browses it.
pub struct AdminUserService;

impl AdminUserService {
    pub const DEFAULT_PAGE_SIZE: u32 = 20;
    pub const MAX_PAGE_SIZE: u32 = 100;

    /// A page size no caller can use to ask for the whole table.
    ///
    /// Zero falls back to the default rather than erroring: it arrives from a query
    /// string, and a request for no rows is a mistake in the caller rather than a policy
    /// worth honouring with an empty page.
    pub fn clamp_page_size(requested: Option<u32>) -> u32 {
        match requested {
            Some(0) | None => Self::DEFAULT_PAGE_SIZE,
            Some(size) => size.min(Self::MAX_PAGE_SIZE),
        }
    }

    /// One page of the roster.
    ///
    /// `live` is the set of canonical `game:gamertag` identities holding a voice
    /// connection, passed in rather than read here so this stays a database function and
    /// every row in a page is measured against the same instant.
    pub async fn list<C: ConnectionTrait>(
        conn: &C,
        defaults: &HashMap<String, bool>,
        live: &HashSet<String>,
        page: u32,
        page_size: u32,
        search: Option<String>,
        game: Option<Game>,
    ) -> Result<PaginatedResponse<AdminUserRow>, DbErr> {
        // A row with no gamertag cannot be banned, permissioned or searched for by name
        // through any admin route, so it is not a row an operator can act on.
        let mut query = player::Entity::find().filter(player::Column::Gamertag.is_not_null());

        if let Some(ref needle) = search {
            let trimmed = needle.trim();
            if !trimmed.is_empty() {
                query = query.filter(player::Column::Gamertag.contains(trimmed));
            }
        }

        if let Some(ref g) = game {
            query = query.filter(player::Column::Game.eq(g.clone()));
        }

        // Ordered by name so a page boundary does not move between two requests, which an
        // id or an insertion order would allow whenever a login writes a row mid-browse.
        let paginator = query
            .order_by_asc(player::Column::Gamertag)
            .paginate(conn, page_size as u64);

        let total = paginator.num_items().await? as u32;
        let models = paginator.fetch_page(page as u64).await?;

        let overrides = Self::overrides_for(conn, &models).await?;
        let none: HashMap<String, i32> = HashMap::new();

        let items = models
            .into_iter()
            .map(|model| {
                let permissions =
                    Self::effective(defaults, overrides.get(&model.id).unwrap_or(&none));
                let gamertag = model.gamertag.unwrap_or_default();
                let identity = model.game.membership_key(&gamertag);
                AdminUserRow {
                    connected: live.contains(&identity),
                    gamertag,
                    game: model.game,
                    banished: model.banished,
                    permissions,
                    created_at: model.created_at,
                }
            })
            .collect();

        Ok(PaginatedResponse {
            items,
            total,
            page,
            page_size,
        })
    }

    /// Every override held by the players on this page, in one query.
    ///
    /// `PermissionService::evaluate_all` answers for a single player, so calling it per
    /// row would issue one query per player: twenty on a default page.
    async fn overrides_for<C: ConnectionTrait>(
        conn: &C,
        models: &[player::Model],
    ) -> Result<HashMap<i32, HashMap<String, i32>>, DbErr> {
        if models.is_empty() {
            return Ok(HashMap::new());
        }

        let ids: Vec<i32> = models.iter().map(|m| m.id).collect();
        let rows = player_permission::Entity::find()
            .filter(player_permission::Column::PlayerId.is_in(ids))
            .all(conn)
            .await?;

        let mut by_player: HashMap<i32, HashMap<String, i32>> = HashMap::new();
        for row in rows {
            by_player
                .entry(row.player_id)
                .or_default()
                .insert(row.permission, row.effect);
        }
        Ok(by_player)
    }

    /// The same rule `PermissionService::evaluate_all` applies: an override decides,
    /// otherwise the server default, otherwise denied.
    fn effective(
        defaults: &HashMap<String, bool>,
        overrides: &HashMap<String, i32>,
    ) -> Vec<Permission> {
        Permission::all()
            .into_iter()
            .filter(|permission| match overrides.get(permission.as_str()) {
                Some(effect) => effect & 1 == 1,
                None => *defaults.get(permission.as_str()).unwrap_or(&false),
            })
            .collect()
    }
}
