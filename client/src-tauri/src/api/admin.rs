use log::error;
use reqwest::{
    Method, StatusCode,
    header::{HeaderMap, HeaderValue},
};
use serde::Serialize;
use serde::de::DeserializeOwned;

use common::Game;
use common::request::admin::{
    AdminUserListQuery, BanishUserRequest, ClearPermissionRequest, CreateUserRequest,
    SetPermissionRequest,
};
use common::response::PaginatedResponse;
use common::response::admin::{AdminActionOutcome, AdminUserRow, PermissionListResponse};
use common::response::auth::IntrospectResponse;
use common::structs::permission::PermissionEffect;

use super::Api;
use super::circuit_breaker::SendError;

impl Api {
    /// One page of the server roster.
    pub(crate) async fn admin_list_users(
        &self,
        query: &AdminUserListQuery,
    ) -> Result<PaginatedResponse<AdminUserRow>, String> {
        let mut params: Vec<(&str, String)> = Vec::new();
        if let Some(page) = query.page {
            params.push(("page", page.to_string()));
        }
        if let Some(page_size) = query.page_size {
            params.push(("page_size", page_size.to_string()));
        }
        if let Some(ref search) = query.search {
            params.push(("search", search.clone()));
        }
        if let Some(ref game) = query.game {
            params.push(("game", game.as_str().to_string()));
        }

        self.admin_get(format!("{}/api/admin/user", self.endpoint()), &params)
            .await
    }

    /// The calling identity's own permissions, straight from the server.
    pub(crate) async fn introspect(&self) -> Result<IntrospectResponse, String> {
        self.admin_get(format!("{}/api/auth/introspect", self.endpoint()), &[])
            .await
    }

    /// Whitelist a player. Registration is what admits a login.
    pub(crate) async fn admin_create_user(
        &self,
        gamertag: &str,
        game: &Game,
    ) -> Result<AdminActionOutcome, String> {
        self.admin_mutate(
            Method::POST,
            "/api/admin/user",
            &CreateUserRequest {
                gamertag: gamertag.to_string(),
                game: game.clone(),
            },
        )
        .await
    }

    pub(crate) async fn admin_set_banished(
        &self,
        gamertag: &str,
        game: &Game,
        banish: bool,
    ) -> Result<AdminActionOutcome, String> {
        self.admin_mutate(
            Method::PATCH,
            "/api/admin/user/banish",
            &BanishUserRequest {
                gamertag: gamertag.to_string(),
                game: game.clone(),
                banish,
            },
        )
        .await
    }

    /// A player's explicit overrides, which is what separates a default allow from an
    /// explicit one. The roster row carries only the effective set.
    pub(crate) async fn admin_list_permissions(
        &self,
        gamertag: &str,
        game: &Game,
    ) -> Result<PermissionListResponse, String> {
        let url = self.permission_url(game, gamertag)?;
        self.admin_get(url, &[]).await
    }

    pub(crate) async fn admin_set_permission(
        &self,
        gamertag: &str,
        game: &Game,
        permission: &str,
        effect: PermissionEffect,
    ) -> Result<AdminActionOutcome, String> {
        self.admin_mutate(
            Method::PUT,
            "/api/admin/permission",
            &SetPermissionRequest {
                gamertag: gamertag.to_string(),
                game: game.clone(),
                permission: permission.to_string(),
                effect,
            },
        )
        .await
    }

    pub(crate) async fn admin_clear_permission(
        &self,
        gamertag: &str,
        game: &Game,
        permission: &str,
    ) -> Result<AdminActionOutcome, String> {
        self.admin_mutate(
            Method::DELETE,
            "/api/admin/permission",
            &ClearPermissionRequest {
                gamertag: gamertag.to_string(),
                game: game.clone(),
                permission: permission.to_string(),
            },
        )
        .await
    }

    /// A gamertag goes into a path segment, and Xbox gamertags carry spaces. Built through
    /// `url` so each segment is percent-encoded; a formatted string would truncate at the
    /// first character the server reads as a delimiter.
    fn permission_url(&self, game: &Game, gamertag: &str) -> Result<String, String> {
        let mut url = url::Url::parse(self.endpoint())
            .map_err(|e| format!("Server address is not a URL: {}", e))?;
        {
            let mut segments = url
                .path_segments_mut()
                .map_err(|_| "Server address cannot carry a path".to_string())?;
            segments.extend(["api", "admin", "permission", game.as_str()]);
            segments.push(gamertag);
        }
        Ok(url.to_string())
    }

    /// Every admin GET: send, require 200, parse.
    async fn admin_get<T: DeserializeOwned>(
        &self,
        url: String,
        params: &[(&str, String)],
    ) -> Result<T, String> {
        let client = self.get_reqwest_client();

        let mut headers = HeaderMap::new();
        headers.insert("Accept", HeaderValue::from_static("application/json"));

        match self
            .send(client.get(url).query(params).headers(headers))
            .await
        {
            Ok(response) if response.status() == StatusCode::OK => {
                let body = response
                    .text()
                    .await
                    .map_err(|e| format!("Failed to read response: {}", e))?;
                serde_json::from_str(&body).map_err(|e| format!("Failed to parse response: {}", e))
            }
            Ok(response) => Err(format!("Server returned status: {}", response.status())),
            Err(SendError::Open) => Err("Server temporarily unreachable; backing off".to_string()),
            Err(SendError::Transport(e)) => {
                error!("Admin request failed: {}", e);
                Err(format!("Connection failed: {}", e))
            }
        }
    }

    /// Every admin mutation. The status becomes an outcome the pane can speak about, and
    /// only a transport failure is an error.
    async fn admin_mutate<B: Serialize>(
        &self,
        method: Method,
        path: &str,
        body: &B,
    ) -> Result<AdminActionOutcome, String> {
        let client = self.get_reqwest_client();
        let url = format!("{}{}", self.endpoint(), path);

        let mut headers = HeaderMap::new();
        headers.insert("Accept", HeaderValue::from_static("application/json"));

        match self
            .send(client.request(method, url).headers(headers).json(body))
            .await
        {
            Ok(response) => Ok(Self::outcome_for(response.status())),
            Err(SendError::Open) => Err("Server temporarily unreachable; backing off".to_string()),
            Err(SendError::Transport(e)) => {
                error!("Admin request failed: {}", e);
                Err(format!("Connection failed: {}", e))
            }
        }
    }

    /// A 5xx is `Invalid` rather than an error because the caller has already reached the
    /// server: the change did not happen and the pane says so, which is the same thing it
    /// says for a rejected request.
    fn outcome_for(status: StatusCode) -> AdminActionOutcome {
        match status {
            StatusCode::CONFLICT => AdminActionOutcome::Conflict,
            StatusCode::NOT_FOUND => AdminActionOutcome::NotFound,
            StatusCode::FORBIDDEN | StatusCode::UNAUTHORIZED => AdminActionOutcome::Forbidden,
            s if s.is_success() => AdminActionOutcome::Applied,
            _ => AdminActionOutcome::Invalid,
        }
    }
}
