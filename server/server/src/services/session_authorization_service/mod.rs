use common::curia;
use std::sync::Arc;

use common::structs::certificate::CertificateFingerprint;
use entity::player;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use crate::services::CertificateRevocationService;
use crate::stream::quic::{CertificateCommonName, ConnectionClassifier, ConnectionKind};

mod rejection;

pub use rejection::SessionRejection;

/// Decides whether a presented client certificate may open a voice session.
///
/// Shared by the QUIC handshake and the WebSocket upgrade so both transports admit exactly
/// the same population. Before this existed, each parsed the Common Name and trusted it, so a
/// certificate this CA had ever signed opened a session whether or not the player still
/// existed, was banished, or had been revoked.
pub struct SessionAuthorizationService {
    revocations: Arc<CertificateRevocationService>,
}

impl SessionAuthorizationService {
    pub fn new(revocations: Arc<CertificateRevocationService>) -> Self {
        Self { revocations }
    }

    pub fn new_shared(revocations: Arc<CertificateRevocationService>) -> Arc<Self> {
        Arc::new(Self::new(revocations))
    }

    /// The revocation list this authorizer consults, for callers that also need to write to it.
    pub fn revocations(&self) -> &Arc<CertificateRevocationService> {
        &self.revocations
    }

    /// The player this leaf certificate may act as, or why it may not.
    ///
    /// Deliberately not cached. A connection is established once per client session, so the
    /// cost is one indexed query at handshake — and a cache here would need its own
    /// invalidation on ban, which is a second thing to get wrong.
    pub async fn authorize<C: ConnectionTrait>(
        &self,
        conn: &C,
        leaf_der: &[u8],
    ) -> Result<player::Model, SessionRejection> {
        let fingerprint = CertificateFingerprint::from_der(leaf_der);
        if self.revocations.is_revoked(conn, &fingerprint).await {
            return Err(SessionRejection::Revoked);
        }

        let Some(common_name) = CertificateCommonName::from_der(leaf_der) else {
            return Err(SessionRejection::Unreadable);
        };

        let (game, name) = match ConnectionClassifier::classify(&common_name) {
            ConnectionKind::Player { game, name } => (game, name),
            ConnectionKind::Rejected { .. } => return Err(SessionRejection::NotAPlayer),
        };

        let found = player::Entity::find()
            .filter(player::Column::Gamertag.eq(&name))
            .filter(player::Column::Game.eq(game))
            .one(conn)
            .await;

        match found {
            Ok(Some(player)) if player.banished => Err(SessionRejection::Banished),
            Ok(Some(player)) => Ok(player),
            Ok(None) => Err(SessionRejection::UnknownPlayer),
            Err(e) => {
                curia::error!("session authorization lookup failed: {}", e);
                Err(SessionRejection::Unavailable)
            }
        }
    }

    /// The fingerprint a handshake should carry for this leaf, so a revocation can find the
    /// live connection later.
    pub fn fingerprint(leaf_der: &[u8]) -> String {
        CertificateFingerprint::from_der(leaf_der)
    }
}
