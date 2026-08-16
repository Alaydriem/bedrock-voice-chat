use common::ncryptflib::rocket::Utc;
use entity::{player, player_auth_code};
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
};

mod error;

pub use error::AuthCodeError;

pub struct AuthCodeService;

impl AuthCodeService {
    pub async fn generate_code<C: ConnectionTrait>(
        conn: &C,
        player_id: i32,
        duration_secs: u64,
        ephemeral: bool,
    ) -> Result<String, anyhow::Error> {
        let alphabet: Vec<char> = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789".chars().collect();
        let code = nanoid::nanoid!(8, &alphabet);

        let now = Utc::now().timestamp();
        let expires_at = now + duration_secs as i64;

        let active_model = player_auth_code::ActiveModel {
            code: ActiveValue::Set(code.clone()),
            player_id: ActiveValue::Set(player_id),
            expires_at: ActiveValue::Set(expires_at),
            used: ActiveValue::Set(false),
            ephemeral: ActiveValue::Set(ephemeral),
            created_at: ActiveValue::Set(now),
            updated_at: ActiveValue::Set(now),
            ..Default::default()
        };

        active_model
            .insert(conn)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to insert auth code: {}", e))?;

        Ok(code)
    }

    /// Redeem a code and return the player it was issued for.
    ///
    /// The code is the only credential. It identifies the player through `player_id`, so
    /// this row is the authority on who is signing in and there is nothing for a caller
    /// to cross-check it against.
    pub async fn validate_and_consume_code<C: ConnectionTrait>(
        conn: &C,
        code: &str,
    ) -> Result<player::Model, AuthCodeError> {
        let auth_code = player_auth_code::Entity::find()
            .filter(player_auth_code::Column::Code.eq(code))
            .one(conn)
            .await
            .map_err(|e| AuthCodeError::DatabaseError(e.to_string()))?;

        let auth_code = match auth_code {
            Some(ac) => ac,
            None => return Err(AuthCodeError::CodeNotFound),
        };

        // Check expiration
        let now = Utc::now().timestamp();
        if auth_code.expires_at < now {
            return Err(AuthCodeError::CodeExpired);
        }

        // Single-use only applies to ephemeral codes: one already redeemed is
        // rejected (fast path). A non-ephemeral code is reusable until expiry.
        if auth_code.ephemeral && auth_code.used {
            return Err(AuthCodeError::CodeAlreadyUsed);
        }

        // Load the related player
        let player_record = player::Entity::find_by_id(auth_code.player_id)
            .one(conn)
            .await
            .map_err(|e| AuthCodeError::DatabaseError(e.to_string()))?;

        let player_record = match player_record {
            Some(p) => p,
            None => return Err(AuthCodeError::PlayerNotFound),
        };

        // Ephemeral codes are atomically consumed: only the redemption that flips
        // used false->true wins, so a concurrent or repeat redemption updates zero
        // rows and is rejected (closes the check-then-act race). Non-ephemeral
        // codes are left intact for reuse until they expire.
        if auth_code.ephemeral {
            let consumed = player_auth_code::Entity::update_many()
                .col_expr(player_auth_code::Column::Used, Expr::value(true))
                .filter(player_auth_code::Column::Code.eq(code))
                .filter(player_auth_code::Column::Used.eq(false))
                .exec(conn)
                .await
                .map_err(|e| AuthCodeError::DatabaseError(e.to_string()))?;

            if consumed.rows_affected == 0 {
                return Err(AuthCodeError::CodeAlreadyUsed);
            }
        }

        Ok(player_record)
    }
}
