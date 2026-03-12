use common::ncryptflib::rocket::Utc;
use entity::{player, player_auth_code};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
};

#[derive(Debug)]
pub enum AuthCodeError {
    CodeNotFound,
    CodeExpired,
    CodeAlreadyUsed,
    GamertagMismatch,
    PlayerNotFound,
    DatabaseError(String),
}

impl std::fmt::Display for AuthCodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthCodeError::CodeNotFound => write!(f, "Auth code not found"),
            AuthCodeError::CodeExpired => write!(f, "Auth code has expired"),
            AuthCodeError::CodeAlreadyUsed => write!(f, "Auth code has already been used"),
            AuthCodeError::GamertagMismatch => write!(f, "Gamertag does not match auth code"),
            AuthCodeError::PlayerNotFound => write!(f, "Player not found for auth code"),
            AuthCodeError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
        }
    }
}

impl std::error::Error for AuthCodeError {}

pub struct AuthCodeService;

impl AuthCodeService {
    pub async fn generate_code<C: ConnectionTrait>(
        conn: &C,
        player_id: i32,
        duration_secs: u64,
    ) -> Result<String, anyhow::Error> {
        let alphabet: Vec<char> = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789".chars().collect();
        let code = nanoid::nanoid!(8, &alphabet);

        let now = Utc::now().timestamp() as u32;
        let expires_at = now + duration_secs as u32;

        let active_model = player_auth_code::ActiveModel {
            code: ActiveValue::Set(code.clone()),
            player_id: ActiveValue::Set(player_id),
            expires_at: ActiveValue::Set(expires_at),
            used: ActiveValue::Set(false),
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

    pub async fn validate_and_consume_code<C: ConnectionTrait>(
        conn: &C,
        code: &str,
        gamertag: &str,
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
        let now = Utc::now().timestamp() as u32;
        if auth_code.expires_at < now {
            return Err(AuthCodeError::CodeExpired);
        }

        // Used check (disabled for now)
        // if auth_code.used {
        //     return Err(AuthCodeError::CodeAlreadyUsed);
        // }

        // Load the related player
        let player_record = player::Entity::find_by_id(auth_code.player_id)
            .one(conn)
            .await
            .map_err(|e| AuthCodeError::DatabaseError(e.to_string()))?;

        let player_record = match player_record {
            Some(p) => p,
            None => return Err(AuthCodeError::PlayerNotFound),
        };

        // Verify gamertag matches
        match &player_record.gamertag {
            Some(gt) if gt == gamertag => {}
            _ => return Err(AuthCodeError::GamertagMismatch),
        }

        // Mark as used (disabled for now)
        // let mut active_model: player_auth_code::ActiveModel = auth_code.into();
        // active_model.used = ActiveValue::Set(true);
        // active_model
        //     .update(conn)
        //     .await
        //     .map_err(|e| AuthCodeError::DatabaseError(e.to_string()))?;

        Ok(player_record)
    }
}
