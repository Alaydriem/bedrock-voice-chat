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
    private val identityOf: (UUID) -> String?,
    private val channelFactory: (VoicechatServerApi) -> SvcChannelFactory
) {
    @Volatile
    private var peer: BvcPeer? = null

    @Volatile
    private var pump: Thread? = null

    @Volatile
    private var refresher: Thread? = null

    /**
     * Builds the plugin SVC will register.
     *
     * `serverPeerlink` is the BVC server this bridge dials. Null means the operator
     * has not granted this bridge yet, in which case the bridge still registers —
     * so SVC keeps working — and logs what they need to paste.
     */
    fun bridge(serverPeerlink: String?): SvcBridge {
        if (serverPeerlink.isNullOrBlank()) {
            logger.warn(
                "Simple Voice Chat is present but this bridge is not peered yet. " +
                    "Add this block to the BVC server's config.hcl, then set " +
                    "svc-bridge-peerlink in the mod config:\n{}",
                peering.grantBlock()
            )
        }

        val outbound = OutboundTranslator(relayWorld, speakers)

        return SvcBridge(
            outbound = outbound,
            onFrame = { frame -> send(frame) },
            onServerApi = { api -> onServerApi(api, serverPeerlink) }
        )
    }

    private fun send(frame: uniffi.bvc_relay_sdk.SdkFrame) {
        val session = peer ?: return
        try {
            session.send(frame)
        } catch (e: Exception) {
            // Not connected yet, or the link dropped. The session redials on its
            // own, and a speaker talking through an outage would otherwise log per
            // frame.
            logger.debug("Dropping an outbound frame: {}", e.toString())
        }
    }

    private fun onServerApi(api: VoicechatServerApi, serverPeerlink: String?) {
        SvcCategories.register(api)

        if (serverPeerlink.isNullOrBlank()) {
            return
        }

        // Resolved per delivery rather than per channel, so a player who opens or
        // closes their BVC client mid-session is handled without reopening anything.
        val speakerFilter = SvcSpeakers { listener ->
            identityOf(listener)?.let { liveClients.isLive(it) } ?: false
        }

        startRefreshing()

        val channels = SvcChannels(channelFactory(api), speakerFilter)
        val inbound = InboundTranslator(channels, SampleRateGuard())

        val session = runBlocking {
            BvcPeer.open(
                SdkConfig(
                    nodeDir = nodeDir.absolutePath,
                    peerlink = serverPeerlink,
                    worlds = listOf(relayWorld.id()),
                    relayUrl = null,
                    inboxCapacity = INBOX_CAPACITY
                )
            )
        }
        peer = session

        // `nextFrame` parks when idle and returns null only once the session is
        // closed, so this is a blocking read rather than a poll.
        pump = Thread({
            runBlocking {
                while (true) {
                    val frame = session.nextFrame() ?: break
                    inbound.inject(frame)
                }
            }
            logger.info("Simple Voice Chat bridge session ended")
        }, "bvc-svc-inbound").apply {
            isDaemon = true
            start()
        }

        logger.info("Simple Voice Chat bridge peered, declaring relay world {}", relayWorld.id())
    }

    /**
     * Keeps the external snapshot current.
     *
     * Only the polled source needs this; embedded answers per call. The interval is
     * a compromise the failure mode forgives: a player who has just connected their
     * BVC client may hear a speaker twice for up to that long, which is briefly
     * annoying, where suppressing too eagerly would be silence.
     */
    private fun startRefreshing() {
        if (!liveClients.isPolled || refresher != null) {
            return
        }

        refresher = Thread({
            while (!Thread.currentThread().isInterrupted) {
                liveClients.refresh()
                try {
                    Thread.sleep(REFRESH_INTERVAL_MS)
                } catch (e: InterruptedException) {
                    Thread.currentThread().interrupt()
                }
            }
        }, "bvc-svc-live-clients").apply {
            isDaemon = true
            start()
        }
    }

    fun shutdown() {
        refresher?.interrupt()
        refresher = null

        val session = peer ?: return
        peer = null

        // Shuts the session down before joining, because the pump is parked in
        // `nextFrame` and only an explicit shutdown releases it.
        runBlocking { session.shutdown() }
        pump?.join(SHUTDOWN_WAIT_MS)
        pump = null
    }

    companion object {
        private const val INBOX_CAPACITY: UInt = 64u

        private const val SHUTDOWN_WAIT_MS: Long = 2000

        private const val REFRESH_INTERVAL_MS: Long = 2000

        private val logger = LoggerFactory.getLogger("BVC SVC")
    }
}
