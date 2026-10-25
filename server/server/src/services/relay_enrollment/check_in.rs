use std::sync::Arc;
use std::time::Duration;

use common::curia;
use tokio::task::JoinHandle;
use tokio::time::MissedTickBehavior;
use tokio_util::sync::CancellationToken;

use super::client::RelayEnrollmentClient;
use super::nonce::CurrentNonce;

const CHECK_IN_INTERVAL: Duration = Duration::from_secs(3600);
const CHECK_IN_TIMEOUT: Duration = Duration::from_secs(30);

pub struct RelayCheckIn;

impl RelayCheckIn {
    pub fn spawn(
        client: Arc<RelayEnrollmentClient>,
        address: Option<String>,
        port: u16,
        nonce: Arc<CurrentNonce>,
        cancel: CancellationToken,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(CHECK_IN_INTERVAL);
            interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => break,
                    _ = interval.tick() => {
                        let exchange = tokio::time::timeout(
                            CHECK_IN_TIMEOUT,
                            client.check_in(address.as_deref(), port),
                        );
                        tokio::select! {
                            _ = cancel.cancelled() => break,
                            result = exchange => match result {
                                Ok(Ok(value)) => nonce.set(value),
                                Ok(Err(e)) => curia::warn!("relay check-in failed", { "error": e.to_string() }),
                                Err(_) => curia::warn!("relay check-in timed out"),
                            },
                        }
                    }
                }
            }
        })
    }
}
