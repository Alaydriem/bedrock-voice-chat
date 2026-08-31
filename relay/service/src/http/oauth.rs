use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use base64::Engine;
use common::curia;
use serde::Deserialize;

use crate::discord::DiscordOAuthClient;

use super::state::HttpState;

// Set on the registry's own origin and compared at the callback.
//
// The `__Host-` prefix binds it to this exact host with no Domain attribute, which is
// what stops a sibling subdomain setting it. `SameSite=Lax` still arrives on the
// top-level GET Discord redirects with, and nothing weaker is needed because this is
// the only cookie in the flow.
pub const STATE_COOKIE: &str = "__Host-bvc_state";

const STATE_BYTES: usize = 32;

#[derive(Deserialize)]
pub struct CallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
}

pub struct OAuthRoutes;

impl OAuthRoutes {
    pub async fn start(State(state): State<Arc<HttpState>>) -> Response {
        let value = Self::mint_state();
        let url = DiscordOAuthClient::authorize_url(
            &state.discord.client_id,
            &state.http.redirect_uri(),
            &value,
        );

        let cookie =
            format!("{STATE_COOKIE}={value}; Path=/; Secure; HttpOnly; SameSite=Lax; Max-Age=600");

        (
            StatusCode::SEE_OTHER,
            [(header::LOCATION, url), (header::SET_COOKIE, cookie)],
        )
            .into_response()
    }

    pub async fn callback(
        State(state): State<Arc<HttpState>>,
        headers: HeaderMap,
        Query(query): Query<CallbackQuery>,
    ) -> Response {
        let (Some(code), Some(supplied)) = (query.code, query.state) else {
            return (StatusCode::BAD_REQUEST, "missing code or state").into_response();
        };

        let Some(expected) = Self::cookie_state(&headers) else {
            return (StatusCode::BAD_REQUEST, "missing state cookie").into_response();
        };

        // Compared rather than trusted. Without this anyone could hand the callback a
        // code of their choosing and have it enrolled against whoever clicked.
        if expected != supplied {
            return (StatusCode::BAD_REQUEST, "state mismatch").into_response();
        }

        let discord_user_id = match state.identity.identify(&code).await {
            Ok(id) => id,
            Err(e) => {
                curia::warn!(format!("a Discord code exchange failed: {e}"));
                return Self::redirect(&state, "error=exchange_failed");
            }
        };

        match state.registry.issue_token(&discord_user_id).await {
            Ok(token) => match state.claims.store(&token).await {
                Ok(id) => Self::redirect(&state, &format!("claim={id}")),
                Err(e) => {
                    curia::error!(format!("storing a claim failed: {e}"));
                    Self::redirect(&state, "error=internal")
                }
            },
            Err(e) => {
                curia::info!("refusing an enrollment request", { "reason": e.to_string() });
                Self::redirect(&state, &format!("error={}", Self::reason(&e)))
            }
        }
    }

    // A redirect rather than a status code. The person is in a browser, and an API
    // error would show them a blank page instead of what to do next.
    fn redirect(state: &HttpState, query: &str) -> Response {
        (
            StatusCode::SEE_OTHER,
            [(
                header::LOCATION,
                format!("{}/enrolled?{query}", state.http.page_origin),
            )],
        )
            .into_response()
    }

    fn reason(error: &crate::registry::RegistryError) -> &'static str {
        use crate::registry::RegistryError;
        match error {
            RegistryError::NotEntitled => "not_entitled",
            RegistryError::AlreadyRegistered => "already_registered",
            _ => "internal",
        }
    }

    fn cookie_state(headers: &HeaderMap) -> Option<String> {
        headers
            .get(header::COOKIE)?
            .to_str()
            .ok()?
            .split(';')
            .filter_map(|pair| pair.trim().split_once('='))
            .find(|(name, _)| *name == STATE_COOKIE)
            .map(|(_, value)| value.to_string())
    }

    fn mint_state() -> String {
        let mut bytes = [0u8; STATE_BYTES];
        getrandom::fill(&mut bytes).expect("the system random source is available");
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
    }
}
