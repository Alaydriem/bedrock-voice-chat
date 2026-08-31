package com.alaydriem.bedrockvoicechat.svc

import de.maxhenkel.voicechat.api.VoicechatServerApi
import kotlinx.coroutines.runBlocking
import org.slf4j.LoggerFactory
import uniffi.bvc_relay_sdk.BvcPeer
import uniffi.bvc_relay_sdk.SdkConfig
import java.io.File
import java.util.UUID

/**
 * Owns the bridge's session and pumps frames in both directions.
 *
 * Assembled here rather than in each platform entry point, because the only
 * platform-shaped parts are the two lookups and the channel factory. Everything
 * else — the peer session, the translators, the pump — is the same on Paper and
 * Fabric.
 */
class SvcBridgeHost(
    private val relayWorld: RelayWorld,
    private val peering: BridgePeering,
    private val nodeDir: File,
    private val speakers: (UUID) -> SpeakerSnapshot?,
    private val liveClients: LiveClients,
    private val identitiesOf: (UUID) -> List<String>,
    private val onlinePlayers: () -> List<UUID>,
    private val onServerThread: (Runnable) -> Unit,
    private val channelFactory: (VoicechatServerApi) -> SvcChannelFactory
) {
    @Volatile
    private var peer: BvcPeer? = null

    @Volatile
    private var reconciler: Thread? = null

    @Volatile
    private var serverApi: VoicechatServerApi? = null

    // Held so a later attempt can ask again. The first one runs while the bridge may still
    // be unpaired, and the answer it gets then is not the answer it gets after a redemption.
    @Volatile
    private var peerlinkSource: (() -> String?)? = null

    // Admits one connect attempt at a time. It stays held for the life of the session,
    // because `openSession` blocks in the inbound loop until the session ends, so it also
    // answers "is there a session" — and clears on the way out, which is what lets a
    // redemption open one after an earlier attempt gave up.
    private val connecting = java.util.concurrent.atomic.AtomicBoolean(false)

    private val announcedFirstFrame = java.util.concurrent.atomic.AtomicBoolean(false)

    /**
     * Builds the plugin SVC will register.
     *
     * `serverPeerlink` is the BVC server this bridge dials. Null means the operator has
     * not paired this bridge yet, in which case the bridge still registers — so SVC keeps
     * working — and logs the command that pairs it. It is asked again rather than read
     * once, because [onPaired] answers it a second time on a server that pairs while it is
     * running.
     */
    fun bridge(serverPeerlink: () -> String?): SvcBridge {
        val outbound = OutboundTranslator(relayWorld, speakers)

        return SvcBridge(
            outbound = outbound,
            onFrame = { frame -> send(frame) },
            onServerApi = { api -> onServerApi(api, serverPeerlink) }
        )
    }

    /**
     * Whether Simple Voice Chat currently holds a voice connection for this player.
     *
     * Asked rather than inferred from their packets. Packets answer who is speaking, so a
     * connected player who is listening would read as having no voice connection at all,
     * and would flip back to it every time they stopped talking.
     *
     * Re-fetched per call because a VoicechatConnection documents itself as a snapshot that
     * does not track the connection it came from.
     *
     * A player with a live BVC client is excluded, because [SvcPresence] is what put the
     * connected mark on them. Reading it back would return our own claim to the BVC server
     * as though Simple Voice Chat had made it, and the moment their BVC client closed they
     * would be reported as holding a bridged connection that never existed.
     */
    fun isOnVoice(player: UUID): Boolean =
        serverApi?.getConnectionOf(player)?.isConnected == true && !hasLiveBvcClient(player)

    /**
     * Any of a player's identities holding a connection means the player is on one.
     * A linked Bedrock player is known by two names and their BVC client registers
     * under whichever the Xbox Live login carried, which the mod cannot predict.
     */
    private fun hasLiveBvcClient(player: UUID): Boolean =
        identitiesOf(player).any(liveClients::isLive)

    private fun send(frame: uniffi.bvc_relay_sdk.SdkFrame) {
        val session = peer ?: return
        try {
            session.send(frame)

            // Once, so a silent direction can be told apart from a rejected one.
            // Without it, "no audio reaches BVC" looks identical whether Simple
            // Voice Chat never delivered a packet or the far side refused every
            // frame, and those have opposite fixes.
            if (announcedFirstFrame.compareAndSet(false, true)) {
                logger.info(
                    "First voice frame sent to BVC: speaker={} world={} dimension={}",
                    frame.speaker,
                    frame.world,
                    frame.dimension
                )
            }
        } catch (e: Exception) {
            // Not connected yet, or the link dropped. The session redials on its
            // own, and a speaker talking through an outage would otherwise log per
            // frame.
            logger.debug("Dropping an outbound frame: {}", e.toString())
        }
    }

    /**
     * Runs on SVC's event thread, so it does no blocking work itself.
     *
     * The peerlink is resolved on the background thread rather than here, because an
     * embedded server binds its peer endpoint asynchronously: `BvcServerManager.start`
     * spawns the server thread and waits 100 ms, which is not a guarantee the plane
     * exists. Reading it eagerly would usually succeed on a fast machine and return
     * null on a loaded one, leaving the bridge silently unpeered.
     */
    private fun onServerApi(api: VoicechatServerApi, serverPeerlink: () -> String?) {
        serverApi = api
        peerlinkSource = serverPeerlink
        SvcCategories.register(api)

        connect()
    }

    /**
     * Opens the session a redeemed pairing code has just made possible.
     *
     * Simple Voice Chat hands over its API once, at startup, and the connect attempt that
     * follows ends within fifteen seconds on a server that has not been granted yet. That
     * attempt is what starts both halves of the bridge — the frame pump and the presence
     * sweep — so without this an operator who pairs on a running server gets neither: no
     * audio crosses, and every player on a BVC client keeps the disconnected mark Simple
     * Voice Chat puts on anyone it holds no connection for.
     *
     * A no-op while a session is already open, so pairing twice does not open two.
     */
    fun onPaired() {
        connect()
    }

    private fun connect() {
        val api = serverApi ?: return
        val serverPeerlink = peerlinkSource ?: return

        if (!connecting.compareAndSet(false, true)) {
            return
        }

        Thread({
            try {
                openSession(api, serverPeerlink)
            } finally {
                connecting.set(false)
            }
        }, "bvc-svc-connect").apply {
            isDaemon = true
            start()
        }
    }

    private fun openSession(api: VoicechatServerApi, serverPeerlink: () -> String?) {
        val link = awaitPeerlink(serverPeerlink)
        if (link == null) {
            logger.info(
                "Simple Voice Chat is present but this bridge is not paired. " +
                    "Run `bvc-server relay pair` on the BVC server, then " +
                    "/bvc peer <code> at this server's console."
            )
            return
        }

        // Resolved per delivery rather than per channel, so a player who opens or
        // closes their BVC client mid-session is handled without reopening anything.
        val speakerFilter = SvcSpeakers(::hasLiveBvcClient)

        startReconciling(api)

        val channels = SvcChannels(channelFactory(api), speakerFilter)
        val inbound = InboundTranslator(channels, SampleRateGuard())

        val session = runBlocking {
            BvcPeer.open(
                SdkConfig(
                    nodeDir = nodeDir.absolutePath,
                    peerlink = link,
                    worlds = listOf(relayWorld.id()),
                    inboxCapacity = INBOX_CAPACITY
                )
            )
        }
        peer = session

        // `open` returns once the endpoint is bound and the dial loop is spawned, not
        // once a link exists. Claiming to be peered there described this side of the
        // bridge only, and it is the one line an operator is told to look for.
        logger.info("Simple Voice Chat bridge connecting to BVC")

        if (awaitConnected(session)) {
            logger.info(
                "Simple Voice Chat bridge peered, declaring relay world {}",
                relayWorld.id()
            )
        } else {
            // Not a failure. The dial loop keeps retrying and frames flow the moment one
            // lands, so this reports the state rather than abandoning the session.
            logger.warn(
                "Simple Voice Chat bridge is not peered yet and is still dialling BVC; " +
                    "declaring relay world {}. Voice does not cross the bridge until it " +
                    "connects.",
                relayWorld.id()
            )
        }

        // `nextFrame` parks when idle and returns null only once the session is
        // closed, so this is a blocking read rather than a poll.
        runBlocking {
            while (true) {
                val frame = session.nextFrame() ?: break
                inbound.inject(frame)
            }
        }
        logger.info("Simple Voice Chat bridge session ended")
    }

    /**
     * Waits for the dial loop to actually carry a link.
     *
     * Bounded by one dial budget: the session gives a dial fifteen seconds before it
     * abandons that attempt and backs off, so waiting longer here reports nothing a
     * reader could not already infer from the silence.
     */
    private fun awaitConnected(session: BvcPeer): Boolean {
        repeat(CONNECT_ATTEMPTS) {
            if (session.isConnected()) return true

            try {
                Thread.sleep(CONNECT_RETRY_MS)
            } catch (e: InterruptedException) {
                Thread.currentThread().interrupt()
                return false
            }
        }
        return session.isConnected()
    }

    /**
     * Waits for the far side to have a link to give.
     *
     * An external server's link is configuration and is either there or not, so this
     * returns on the first attempt. An embedded one is minted from a live endpoint
     * that may still be binding, which is what the retries are for.
     */
    private fun awaitPeerlink(serverPeerlink: () -> String?): String? {
        repeat(PEERLINK_ATTEMPTS) { attempt ->
            serverPeerlink()?.takeIf { it.isNotBlank() }?.let { return it }

            if (attempt < PEERLINK_ATTEMPTS - 1) {
                try {
                    Thread.sleep(PEERLINK_RETRY_MS)
                } catch (e: InterruptedException) {
                    Thread.currentThread().interrupt()
                    return null
                }
            }
        }
        return null
    }

    /**
     * Keeps both views of who is on a BVC client current.
     *
     * Refreshing the snapshot is the polled source's need alone; embedded answers per
     * call. Reconciling the connected mark is needed either way, because Simple Voice
     * Chat resets it whenever a player's real state changes.
     *
     * The interval is a compromise the failure mode forgives on both counts: a player
     * who has just opened their BVC client may hear a speaker twice for up to that
     * long, and may wear the disconnected mark for as long again. Suppressing or
     * marking more eagerly costs silence and a wrong mark respectively.
     */
    private fun startReconciling(api: VoicechatServerApi) {
        if (reconciler != null) {
            return
        }

        val presence = SvcPresence(
            onlinePlayers = onlinePlayers,
            hasLiveBvcClient = ::hasLiveBvcClient,
            setConnected = { player, connected ->
                api.getConnectionOf(player)?.setConnected(connected)
            }
        )

        reconciler = Thread({
            while (!Thread.currentThread().isInterrupted) {
                if (liveClients.isPolled) {
                    liveClients.refresh()
                }

                // Posted rather than run here: the sweep reads the online player
                // list, which belongs to the server thread.
                try {
                    onServerThread(Runnable { presence.reconcile() })
                } catch (e: Exception) {
                    // A server on its way down refuses work. Nothing to correct
                    // and nobody left to see the mark.
                    logger.debug("Skipping a presence sweep: {}", e.toString())
                }

                try {
                    Thread.sleep(RECONCILE_INTERVAL_MS)
                } catch (e: InterruptedException) {
                    Thread.currentThread().interrupt()
                }
            }
        }, "bvc-svc-presence").apply {
            isDaemon = true
            start()
        }
    }

    fun shutdown() {
        reconciler?.interrupt()
        reconciler = null
        serverApi = null
        peerlinkSource = null

        val session = peer ?: return
        peer = null

        // The connect thread is parked in `nextFrame`, and only an explicit shutdown
        // releases it. Interrupting would not: the park is inside the native session,
        // not in a Java wait.
        runBlocking { session.shutdown() }
    }

    companion object {
        private const val INBOX_CAPACITY: UInt = 64u

        private const val RECONCILE_INTERVAL_MS: Long = 2000

        // Fifteen seconds in total. An embedded server that has not bound a peer
        // endpoint by then is not slow, it is misconfigured — and the warning that
        // follows tells the operator what to add.
        private const val PEERLINK_ATTEMPTS: Int = 15

        private const val PEERLINK_RETRY_MS: Long = 1000

        // One dial budget. `PeerSession` gives a dial fifteen seconds before it gives
        // up on that attempt and backs off, so this waits exactly as long as the first
        // attempt can take and no longer.
        private const val CONNECT_ATTEMPTS: Int = 15

        private const val CONNECT_RETRY_MS: Long = 1000

        private val logger = LoggerFactory.getLogger("BVC SVC")
    }
}
