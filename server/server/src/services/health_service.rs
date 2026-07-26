use std::sync::Arc;
use std::time::Duration;

use sea_orm::DatabaseConnection;

use crate::http::dtos::health::ReadinessResponse;
use crate::runtime::ReadinessState;

/// A hung pool must fail the probe rather than stall the kubelet: bound the
/// ping instead of inheriting the connection timeout.
const DB_PING_TIMEOUT: Duration = Duration::from_secs(2);

const COMPONENT_OK: &str = "ok";
const COMPONENT_DOWN: &str = "down";

/// Evaluates component readiness for the /health/readiness route.
pub struct HealthService {
    readiness: Arc<ReadinessState>,
    cert_path: String,
}

impl HealthService {
    pub fn new(readiness: Arc<ReadinessState>, cert_path: String) -> Self {
        Self {
            readiness,
            cert_path,
        }
    }

    pub fn new_shared(readiness: Arc<ReadinessState>, cert_path: String) -> Arc<Self> {
        Arc::new(Self::new(readiness, cert_path))
    }

    pub async fn evaluate(&self, conn: &DatabaseConnection) -> ReadinessResponse {
        let database = match tokio::time::timeout(DB_PING_TIMEOUT, conn.ping()).await {
            Ok(Ok(())) => COMPONENT_OK,
            _ => COMPONENT_DOWN,
        };
        let quic = if self.readiness.quic_ready() {
            COMPONENT_OK
        } else {
            COMPONENT_DOWN
        };
        // The active HTTPS cert (manual or ACME) must be parseable and
        // unexpired. Zero remaining margin is enough here — renewal margins
        // are the ACME task's job, not the probe's.
        let certificate = match std::fs::read_to_string(&self.cert_path) {
            Ok(pem) => match crate::services::acme::CertificateExpiry::is_valid_for(
                &pem,
                Duration::ZERO,
            ) {
                Ok(true) => COMPONENT_OK,
                _ => COMPONENT_DOWN,
            },
            Err(_) => COMPONENT_DOWN,
        };
        ReadinessResponse {
            database: database.to_string(),
            quic: quic.to_string(),
            certificate: certificate.to_string(),
        }
    }
}
