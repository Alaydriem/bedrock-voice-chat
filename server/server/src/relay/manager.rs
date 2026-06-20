use std::sync::{Arc, Mutex};
use std::time::Duration;

use sea_orm::DatabaseConnection;

use common::structs::relay::RelayEndpoint;

use crate::services::{AudioStreamTokenCache, CertificateService};
use crate::stream::quic::{CacheManager, WebhookReceiver};

use super::announce_task::{ActiveWorldsSource, FnActiveWorldsSource, RelayAnnounceTask};
use super::audio::file_existence::DbAudioFileExistence;
use super::audio::source::AudioSource;
use super::code_crypto::RelayCodeKeypair;
use super::delivery::BroadcastInjectDelivery;
use super::discovery::client::RelayClient;
use super::observe::{CodeDecryptor, ObservedCodeHandler, ProductionObservedCodeHandler};
use super::offer_delivery::ProductionOfferDelivery;
use super::orchestrator::{LocalInjectDelivery, RelayOrchestrator};
use super::peer::dial::driver::{ProductionPeerDialDriver, RedeemedDial};
use super::peer::link::ingest_sink::WebhookIngestSink;
use super::peer::manager::PeerManager;
use super::peer::table::PeerTable;
use super::peer_identity::{ServerPeerStore, StorePresenceGate};
use super::presence::gate::PresenceGate;

// Inputs the runtime feeds to assemble the cross-server voice relay client plane.
pub struct RelayManagerConfig {
    pub self_endpoint: RelayEndpoint,
    pub webhook_receiver: WebhookReceiver,
    pub cache_manager: CacheManager,
    pub cert_service: Arc<CertificateService>,
    pub ca_pem: String,
    pub db_conn: Arc<DatabaseConnection>,
    pub audio_storage_path: String,
    pub audio_stream_token_cache: AudioStreamTokenCache,
    // Self-announce cadence; `None` keeps the production default (60s).
    pub announce_interval: Option<Duration>,
    // Presence/dial/idle-sweep cadence; `None` keeps the production default.
    pub orchestration_interval: Option<Duration>,
    // Idle-link teardown window; `None` keeps the production default (300s).
    pub idle_timeout: Option<Duration>,
}

// Owns the full cross-server voice relay client plane: relay client, peer table,
// the in-memory server-peer identity store + the authorization gate it backs,
// peer manager (with the jukebox audio source wired), register-nonce store, the
// periodic register/lookup background task, the peer dial driver, and the
// dial/idle orchestrator. Construction builds every component; `start` spawns the
// dedicated background + orchestration tasks. All relay work runs off the audio
// hot path.
pub struct RelayManager {
    peer_manager: Arc<PeerManager>,
    server_peer_store: Arc<ServerPeerStore>,
    inject_delivery: Arc<dyn LocalInjectDelivery>,
    observe_handler: Arc<dyn ObservedCodeHandler>,
    announce: Mutex<Option<RelayAnnounceTask>>,
    orchestrator: Mutex<Option<RelayOrchestrator>>,
}

impl RelayManager {
    // Assembles all relay-client components. Errors only when the relay client
    // itself cannot be built (e.g. missing SPKI pin for the default relay).
    pub fn new(config: RelayManagerConfig) -> Result<Self, anyhow::Error> {
        let RelayManagerConfig {
            self_endpoint,
            webhook_receiver,
            cache_manager,
            cert_service,
            ca_pem,
            db_conn,
            audio_storage_path,
            audio_stream_token_cache,
            announce_interval,
            orchestration_interval,
            idle_timeout,
        } = config;

        let relay_client = RelayClient::new_shared();

        let peer_table = PeerTable::new_shared();

        // In-memory server-peer identity store: mints/redeems single-use codes and
        // tracks which endpoint is authorized for which world. It backs the live
        // authorization gate below.
        let server_peer_store = ServerPeerStore::new_shared(cert_service.clone(), ca_pem.clone());

        // Authorization gate: a peer may relay a world's audio — in OR
        // out — only while it holds a redeemed, in-grace identity bound to that
        // world. Backs both `is_ingest_authorized` (inbound) and `forward_local`
        // (outbound).
        let presence_gate: Arc<dyn PresenceGate> =
            StorePresenceGate::new_shared(server_peer_store.clone());
        let ingest = WebhookIngestSink::new_shared(webhook_receiver.clone());
        let peer_manager = PeerManager::new_shared(
            self_endpoint.clone(),
            peer_table.clone(),
            ingest,
            presence_gate,
        );

        // Cross-server jukebox responder: answers a peer's `AudioQuery` with a
        // minted stream token when this server holds the file on disk.
        let existence = DbAudioFileExistence::new_shared(db_conn, audio_storage_path);
        let audio_source = AudioSource::new_shared(audio_stream_token_cache, existence);
        peer_manager.set_audio_source(audio_source);
        if let Some(timeout) = idle_timeout {
            peer_manager.set_idle_timeout(timeout);
        }

        let active_worlds: Arc<dyn ActiveWorldsSource> = {
            let cm = cache_manager.clone();
            Arc::new(FnActiveWorldsSource(move || cm.active_relay_worlds()))
        };

        // Realm injection delivery, shared by the `/relay/offer` route's code
        // injection (`inject_delivery()` accessor) and the self-announce task.
        let inject_delivery: Arc<dyn LocalInjectDelivery> =
            BroadcastInjectDelivery::new_shared(webhook_receiver.clone());

        // Periodically injects this server's endpoint into its realms as a
        // suppressed `!bvca` chat; peers observe it and populate their peer tables.
        let announce = match announce_interval {
            Some(interval) => RelayAnnounceTask::new_with_interval(
                inject_delivery.clone(),
                peer_table.clone(),
                self_endpoint.clone(),
                active_worlds,
                interval,
            ),
            None => RelayAnnounceTask::new(
                inject_delivery.clone(),
                peer_table.clone(),
                self_endpoint.clone(),
                active_worlds,
            ),
        };

        // Dial driver for the asker side of Flow 1: takes the link's outbound
        // receiver and spawns the `PeerDialer` run loop with the redeemed
        // credential, so the queue `forward_local` fills is actually drained.
        let dial_driver =
            ProductionPeerDialDriver::new_shared(peer_manager.clone(), server_peer_store.clone());

        // Our sealed-box keypair: its public key is advertised in offers; minters
        // seal the code to it, and only we can unseal what arrives via the realm.
        let code_keypair = RelayCodeKeypair::new_shared();

        // Asker side of Flow 1: when a local client observes an offered (sealed)
        // code in the realm, unseal it with our keypair, redeem it against the
        // offering minter, and dial with the redeemed `server::`-CN credential via
        // the dial driver (`RedeemedDial`).
        let observe_handler: Arc<dyn ObservedCodeHandler> =
            ProductionObservedCodeHandler::new_shared(
                peer_manager.clone(),
                server_peer_store.clone(),
                code_keypair.clone() as Arc<dyn CodeDecryptor>,
                relay_client.clone(),
                dial_driver as Arc<dyn RedeemedDial>,
                self_endpoint.clone(),
            );

        let mut orchestrator = RelayOrchestrator::new(peer_manager.clone());
        if let Some(interval) = orchestration_interval {
            orchestrator.set_interval(interval);
        }
        // Drive the reconnect-grace lifecycle: idle-closed links enter grace and
        // grace-lapsed identities are swept so a dropped peer is re-offered.
        orchestrator.set_server_peer_store(server_peer_store.clone());
        // Asker side of Flow 1: offer a code to each discovered, unauthorized peer
        // we should initiate to (fires `/relay/offer` at the peer), advertising our
        // sealed-box public key so the minter seals the code to us.
        orchestrator.set_offer_delivery(ProductionOfferDelivery::new_shared(
            relay_client,
            self_endpoint.clone(),
            code_keypair.public_key_bytes(),
        ));

        Ok(Self {
            peer_manager,
            server_peer_store,
            inject_delivery,
            observe_handler,
            announce: Mutex::new(Some(announce)),
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

    // The in-memory server-peer identity store. The relay offer route mints codes
    // against it, the redemption path consumes them, and the QUIC accept/teardown
    // path marks identities connected/disconnected.
    pub fn server_peer_store(&self) -> Arc<ServerPeerStore> {
        self.server_peer_store.clone()
    }

    // Realm code-injection delivery the `/relay/offer` route uses to push a minted
    // code to this server's own local client(s) in a world.
    pub fn inject_delivery(&self) -> Arc<dyn LocalInjectDelivery> {
        self.inject_delivery.clone()
    }

    // Asker-side observe handler the QUIC input path routes a local client's
    // observed `!bvcp` code to (Flow 1 link establishment).
    pub fn observe_handler(&self) -> Arc<dyn ObservedCodeHandler> {
        self.observe_handler.clone()
    }

    // Spawns the dedicated self-announce task and the presence/idle orchestration
    // loop. Idempotent: a second call is a no-op once the owned tasks have been
    // taken.
    pub fn start(&self) {
        if let Some(announce) = self
            .announce
            .lock()
            .expect("relay announce task lock poisoned")
            .take()
        {
            tokio::spawn(async move { announce.run().await });
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
