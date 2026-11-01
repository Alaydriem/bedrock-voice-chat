use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use common::curia;
use common::structs::relay::enroll::EnrollRefuseReason;
use tokio::sync::{Notify, watch};
use tokio::task::JoinHandle;
use tokio::time::MissedTickBehavior;
use tokio_util::sync::CancellationToken;

use super::client::RelayEnrollmentClient;
use super::nonce::CurrentNonce;

const CHECK_IN_INTERVAL: Duration = Duration::from_secs(3600);
const CHECK_IN_TIMEOUT: Duration = Duration::from_secs(30);

pub struct RelayCheckIn;

impl RelayCheckIn {
    // The registry answers a second check-in from one node inside this window with the
    // previous nonce and starts no address probe. A server restarted inside the window, on
    // a new address, would therefore keep its old record until the next hourly check-in.
    pub const REGISTRY_WINDOW: Duration = Duration::from_secs(60);

    pub fn spawn(
        client: Arc<RelayEnrollmentClient>,
        address: Option<String>,
        port: u16,
        nonce: Arc<CurrentNonce>,
        cancel: CancellationToken,
    ) -> JoinHandle<()> {
        Self::spawn_schedule(cancel, None, None, move || {
            let client = client.clone();
            let address = address.clone();
            let nonce = nonce.clone();
            async move {
                Self::attempt(&client, address.as_deref(), port, &nonce, None).await;
            }
        })
    }

    // A play server's check-in. Begins once `listening` reads true, because the registry
    // probes the address the moment it hears it. Repeats once `follow_up` after the first, so
    // a check-in the registry answered without probing is made again outside its window.
    //
    // `name_lost` is notified when the registry says it holds no play name for this server.
    pub fn spawn_with_follow_up(
        client: Arc<RelayEnrollmentClient>,
        address: Option<String>,
        port: u16,
        nonce: Arc<CurrentNonce>,
        cancel: CancellationToken,
        follow_up: Duration,
        listening: watch::Receiver<bool>,
        name_lost: Arc<Notify>,
    ) -> JoinHandle<()> {
        Self::spawn_schedule(cancel, Some(follow_up), Some(listening), move || {
            let client = client.clone();
            let address = address.clone();
            let nonce = nonce.clone();
            let name_lost = name_lost.clone();
            async move {
                Self::attempt(&client, address.as_deref(), port, &nonce, Some(&name_lost)).await;
            }
        })
    }

    // The timing, apart from what a check-in does: once at the start (after `start` reads
    // true, when given), once more `follow_up` later, and then hourly until cancelled.
    pub fn spawn_schedule<F, Fut>(
        cancel: CancellationToken,
        follow_up: Option<Duration>,
        start: Option<watch::Receiver<bool>>,
        mut attempt: F,
    ) -> JoinHandle<()>
    where
        F: FnMut() -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        tokio::spawn(async move {
            if let Some(mut start) = start {
                tokio::select! {
                    _ = cancel.cancelled() => return,
                    ready = start.wait_for(|ready| *ready) => {
                        if ready.is_err() {
                            return;
                        }
                    }
                }
            }

            let mut interval = tokio::time::interval(CHECK_IN_INTERVAL);
            interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
            let mut follow_up = follow_up.map(|after| Box::pin(tokio::time::sleep(after)));

            loop {
                tokio::select! {
                    _ = cancel.cancelled() => break,
                    _ = interval.tick() => {
                        tokio::select! {
                            _ = cancel.cancelled() => break,
                            _ = attempt() => {}
                        }
                    }
                    _ = async { follow_up.as_mut().expect("guarded by the precondition").await }, if follow_up.is_some() => {
                        follow_up = None;
                        tokio::select! {
                            _ = cancel.cancelled() => break,
                            _ = attempt() => {}
                        }
                    }
                }
            }
        })
    }

    // One check-in. `name_lost` is `Some` for a play server, which also wants the outcome
    // said plainly: the registry's acceptance is not proof the address works, but its
    // refusal is the reason players cannot find the server by name.
    async fn attempt(
        client: &RelayEnrollmentClient,
        address: Option<&str>,
        port: u16,
        nonce: &CurrentNonce,
        name_lost: Option<&Notify>,
    ) {
        match tokio::time::timeout(CHECK_IN_TIMEOUT, client.check_in(address, port)).await {
            Ok(Ok(value)) => {
                nonce.set(value);
                if name_lost.is_some() {
                    curia::info!("the registry accepted this server's play check-in and is verifying its address");
                }
            }
            Ok(Err(e)) => {
                // Only NoPlayName says the registry released the name. NotRegistered and
                // PlayUnavailable also describe a registry that is mis-configured or older,
                // and clearing the stored name for those would strand a valid certificate.
                let lost = e
                    .reason()
                    .is_some_and(|reason| matches!(reason, EnrollRefuseReason::NoPlayName));
                match (name_lost, lost) {
                    (Some(name_lost), true) => {
                        curia::error!("the registry holds no play name for this server; restart the server to claim a new one", { "error": e.to_string() });
                        name_lost.notify_one();
                    }
                    (Some(_), false) => {
                        curia::error!("the registry refused this server's play check-in; players cannot reach it by name until it accepts", { "error": e.to_string() });
                    }
                    (None, _) => curia::warn!("relay check-in failed", { "error": e.to_string() }),
                }
            }
            Err(_) => curia::warn!("relay check-in timed out"),
        }
    }
}
