use std::sync::{Arc, Mutex};

use sea_orm::DatabaseConnection;

use common::structs::relay::RelayEndpoint;

use crate::services::{AudioStreamTokenCache, CertificateService};
use crate::stream::quic::{CacheManager, WebhookReceiver};

use super::background_task::{ActiveWorldsSource, FnActiveWorldsSource, RelayBackgroundTask};
use super::delivery::{BroadcastInjectDelivery, LinkEchoDelivery};
use super::discovery::client::RelayClient;
use super::discovery::nonce_store::RegisterNonceStore;
use super::orchestrator::RelayOrchestrator;
use super::peer::cert::issuer::PeerCertIssuer;
use super::peer::dial::driver::ProductionPeerDialDriver;
use super::peer::link::ingest_sink::WebhookIngestSink;
use super::peer::manager::PeerManager;
use super::peer::table::PeerTable;
use super::presence::gate::PresenceGate;
use super::presence::PresenceProver;
use super::audio::file_existence::DbAudioFileExistence;
use super::audio::source::AudioSource;

// Inputs the runtime feeds to assemble the cross-server voice relay client plane.
pub struct RelayManagerConfig {
    pub self_endpoint: RelayEndpoint,
    pub client_url: String,
    pub webhook_receiver: WebhookReceiver,
    pub cache_manager: CacheManager,
    pub cert_service: Arc<CertificateService>,
    pub ca_pem: String,
    pub db_conn: Arc<DatabaseConnection>,
    pub audio_storage_path: String,
    pub audio_stream_token_cache: AudioStreamTokenCache,
}

// Owns the full cross-server voice relay client plane: relay client, peer table,
// presence prover, peer manager (with the jukebox audio source + prover wired),
// peer-cert issuer, register-nonce store, the periodic register/lookup background
// task, the peer dial driver, and the presence/idle orchestrator. Construction
// builds every component; `start` spawns the dedicated background + orchestration
// tasks. All relay work runs off the audio hot path.
pub struct RelayManager {
    peer_manager: Arc<PeerManager>,
    nonce_store: Arc<RegisterNonceStore>,
    peer_cert_issuer: Arc<PeerCertIssuer>,
    background: Mutex<Option<RelayBackgroundTask>>,
    orchestrator: Mutex<Option<RelayOrchestrator>>,
}

impl RelayManager {
    // Assembles all relay-client components. Errors only when the relay client
    // itself cannot be built (e.g. missing SPKI pin for the default relay).
    pub fn new(config: RelayManagerConfig) -> Result<Self, anyhow::Error> {
        let RelayManagerConfig {
            self_endpoint,
            client_url,
            webhook_receiver,
            cache_manager,
            cert_service,
            ca_pem,
            db_conn,
            audio_storage_path,
            audio_stream_token_cache,
        } = config;

        let relay_client = RelayClient::new_shared(&client_url)?;

        let peer_table = PeerTable::new_shared();
        let prover = PresenceProver::new_shared();
        let ingest = WebhookIngestSink::new_shared(webhook_receiver.clone());
        let peer_manager = PeerManager::new_shared(
            self_endpoint.clone(),
            peer_table.clone(),
            ingest,
            prover.clone(),
        );
        peer_manager.set_prover(prover.clone());

        // Cross-server jukebox responder: answers a peer's `AudioQuery` with a
        // minted stream token when this server holds the file on disk.
        let existence = DbAudioFileExistence::new_shared(db_conn, audio_storage_path);
        let audio_source = AudioSource::new_shared(audio_stream_token_cache, existence);
        peer_manager.set_audio_source(audio_source);

        // Peer-cert issuer: the acceptor side of the bootstrap, gated on mutual
        // presence proof (the same prover that gates peering).
        let presence_gate: Arc<dyn PresenceGate> = prover.clone();
        let peer_cert_issuer = PeerCertIssuer::new_shared(cert_service, presence_gate, ca_pem);

        // Nonce store backing `/relay/proof` for endpoint-control proof.
        let nonce_store = RegisterNonceStore::new_shared();

        let active_worlds: Arc<dyn ActiveWorldsSource> = {
            let cm = cache_manager.clone();
            Arc::new(FnActiveWorldsSource(move || cm.active_relay_worlds()))
        };
        let background = RelayBackgroundTask::new(
            relay_client.clone(),
            peer_table.clone(),
            self_endpoint.clone(),
            active_worlds,
            nonce_store.clone(),
        );

        // Dial driver: consumes reconcile's dial intents, fetches a peer cert from
        // the acceptor, and spawns the `PeerDialer` run loop so the outbound queue
        // `forward_local` fills is actually drained.
        let dial_driver = ProductionPeerDialDriver::new_shared(
            relay_client,
            peer_manager.clone(),
            self_endpoint.clone(),
        );

        let inject = BroadcastInjectDelivery::new_shared(webhook_receiver);
        let echo = LinkEchoDelivery::new_shared(peer_manager.clone(), prover);
        let orchestrator = RelayOrchestrator::new_with_driver(
            peer_manager.clone(),
            inject,
            echo,
            dial_driver,
        );

        Ok(Self {
            peer_manager,
            nonce_store,
            peer_cert_issuer,
            background: Mutex::new(Some(background)),
            orchestrator: Mutex::new(Some(orchestrator)),
        })
    }

    pub fn new_shared(config: RelayManagerConfig) -> Result<Arc<Self>, anyhow::Error> {
        Ok(Arc::new(Self::new(config)?))
    }

    // The peer manager doubles as the QUIC fan-out routing handle, the
    // connection-registry inbound router, and the playback service's
    // cross-server jukebox discovery handle.
    pub fn peer_manager(&self) -> Arc<PeerManager> {
        self.peer_manager.clone()
    }

    // Nonce store the Rocket manager mounts `/relay/proof` against.
    pub fn nonce_store(&self) -> Arc<RegisterNonceStore> {
        self.nonce_store.clone()
    }

    // Peer-cert issuer the Rocket manager mounts `/relay/peer-cert` against.
    pub fn peer_cert_issuer(&self) -> Arc<PeerCertIssuer> {
        self.peer_cert_issuer.clone()
    }

    // Spawns the dedicated register/lookup background task and the
    // presence/idle orchestration loop. Idempotent: a second call is a no-op once
    // the owned tasks have been taken.
    pub fn start(&self) {
        if let Some(background) = self
            .background
            .lock()
            .expect("relay background task lock poisoned")
            .take()
        {
            tokio::spawn(async move { background.run().await });
        }
        if let Some(orchestrator) = self
            .orchestrator
            .lock()
            .expect("relay orchestrator lock poisoned")
            .take()
        {
            tokio::spawn(async move { orchestrator.run().await });
        }
    }
}
