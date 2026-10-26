use crate::api::{Api, CredentialVerdict};

use crate::api::circuit_breaker::SendError;
use common::Game;
use common::reqwest::{
    StatusCode,
    header::{HeaderMap, HeaderValue},
};
use common::response::GamerpicResponse;
use std::error::Error;
use tauri_plugin_curia::curia;

mod gate;
mod outcome;

pub use gate::GamerpicGate;
pub(crate) use outcome::GamerpicOutcome;

impl Api {
    pub(crate) async fn get_gamerpic(
        &self,
        game: &Game,
        gamertag: &str,
    ) -> Result<GamerpicOutcome, String> {
        // No log line: the refusal was reported once, when the gate closed.
        if self.gamerpic.is_refused() {
            return Err("Gamerpic requests are paused for this server".to_string());
        }

        let none = || {
            GamerpicOutcome::Found(GamerpicResponse {
                gamertag: gamertag.to_string(),
                game: game.clone(),
                gamerpic: None,
            })
        };

        if self.gamerpic.is_miss(game.as_str(), gamertag) {
            return Ok(none());
        }

        let client = self.get_client();

        let mut headers = HeaderMap::new();
        headers.insert("Accept", HeaderValue::from_static("application/json"));

        let url = format!(
            "{}/api/gamerpic/{}/{}",
            self.endpoint,
            game.as_str(),
            gamertag
        );

        match self.send(client.get(url).headers(headers)).await {
            Ok(response) => match response.status() {
                StatusCode::OK => match response.json::<GamerpicResponse>().await {
                    Ok(result) => {
                        if result.gamerpic.is_none() {
                            self.gamerpic.record_miss(game.as_str(), gamertag);
                        }
                        Ok(GamerpicOutcome::Found(result))
                    }
                    Err(e) => {
                        self.gamerpic.record_miss(game.as_str(), gamertag);
                        curia::error!("failed to parse the gamerpic response", {
                            error: e.to_string(),
                        });
                        Err("Failed to parse response".to_string())
                    }
                },
                status @ (StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN) => {
                    self.on_gamerpic_refused(status).await
                }
                status => {
                    self.gamerpic.record_miss(game.as_str(), gamertag);
                    curia::debug!("gamerpic request failed", {
                        connected_server: self.endpoint.clone(),
                        status: status.as_u16(),
                    });
                    Err(format!("Request failed with status: {}", status))
                }
            },
            Err(SendError::Open) => {
                self.gamerpic.record_miss(game.as_str(), gamertag);
                Err("Server temporarily unreachable; backing off".to_string())
            }
            Err(SendError::Transport(e)) => {
                self.gamerpic.record_miss(game.as_str(), gamertag);
                let mut causes = Vec::new();
                let mut source = e.source();
                while let Some(cause) = source {
                    causes.push(cause.to_string());
                    source = cause.source();
                }
                curia::error!("failed to get gamerpic", {
                    error: e.to_string(),
                    causes: causes,
                });
                Err("Network error occurred".to_string())
            }
        }
    }

    /// Close the gate, and let the one caller that closed it ask whether the certificate is
    /// still accepted. A refused gamerpic is evidence, not proof: the probe is what decides.
    async fn on_gamerpic_refused(&self, status: StatusCode) -> Result<GamerpicOutcome, String> {
        let refused = Err(format!("Request failed with status: {}", status));

        if !self.gamerpic.refuse() {
            return refused;
        }

        curia::warn!("gamerpic requests refused; stopping for this session", {
            connected_server: self.endpoint.clone(),
            status: status.as_u16(),
        });

        if !GamerpicGate::refusal_needs_probe(status.as_u16()) {
            return refused;
        }

        match self.verify_credentials().await {
            CredentialVerdict::Rejected => Ok(GamerpicOutcome::CredentialsRejected),
            verdict => {
                curia::warn!("gamerpic refusal probe finished; credentials kept", {
                    connected_server: self.endpoint.clone(),
                    verdict: format!("{:?}", verdict),
                });
                refused
            }
        }
    }
}
