package com.alaydriem.bedrockvoicechat.server

import com.alaydriem.bedrockvoicechat.api.ConfigProvider
import com.alaydriem.bedrockvoicechat.config.ModConfig
import com.alaydriem.bedrockvoicechat.config.generated.EmbeddedServerConfig
import com.alaydriem.bedrockvoicechat.control.ControlSendResult
import com.alaydriem.bedrockvoicechat.native.BvcNative
import com.alaydriem.bedrockvoicechat.native.ChatFfi
import com.google.gson.Gson
import com.sun.jna.Pointer
import org.slf4j.LoggerFactory
import java.nio.file.Files
import java.util.UUID

/**
 * Manages the embedded BVC server lifecycle.
 * Uses JNA to call the native Rust library.
 */
class BvcServerManager(
    private val config: ModConfig,
    private val configProvider: ConfigProvider
) : ChatFfi {
    companion object {
        private val logger = LoggerFactory.getLogger("BVC Server")
        private val GSON = Gson()

        /**
         * Whether the embedded server has a route to a TLS certificate. Either
         * the operator supplied one, or ACME will obtain one; the runtime
         * refuses both at once, so this does not check for that.
         */
        @JvmStatic
        fun canStart(config: EmbeddedServerConfig?): Boolean {
            val tls = config?.server?.tls ?: return false
            val manual = !tls.certificate.isNullOrBlank() && !tls.key.isNullOrBlank()
            return manual || tls.acme != null
        }
    }

    // @Volatile ensures visibility across threads - handle is set in start(), read in multiple places
    @Volatile
    private var handle: Pointer? = null

    /**
     * The access token this server was started with.
     *
     * Generated here when the config does not supply one, and retained because the embedded
     * mod has to authenticate back to its own server — the chat channel is the first thing
     * that needs it.
     */
    var accessToken: String? = null
        private set

    // @Volatile for thread visibility - serverThread is set in start(), checked in isRunning/stop
    @Volatile
    private var serverThread: Thread? = null

    /** The configuration the server resolved, read back after creation. */
    @Volatile
    private var effectiveConfig: EmbeddedServerConfig? = null

    val isRunning: Boolean
        get() = handle != null && serverThread?.isAlive == true

    /**
     * Whether the embedded server resolved telemetry as enabled.
     *
     * A behavioural accessor rather than exposing the resolved config: the value is
     * decided by defaults and `BVC_*` overrides inside the server, so asking it is
     * the only way to know what it actually chose. Absent config reads as disabled,
     * because a report that cannot be shown to be permitted must not be sent.
     */
    val telemetryEnabled: Boolean
        get() = effectiveConfig?.server?.features?.telemetry == true

    /**
     * Start the embedded BVC server.
     * @return true if started successfully, false otherwise
     */
    fun start(): Boolean {
        if (!config.useEmbeddedServer) {
            logger.debug("Embedded server mode not enabled")
            return false
        }

        if (config.legacyKeys.isNotEmpty()) {
            logger.error(
                "embedded-config uses keys from an older layout. The block now mirrors the BVC server configuration."
            )
            for (key in config.legacyKeys) {
                logger.error("  {}", key)
            }
            return false
        }

        val configDir = configProvider.getConfigDir()
        if (configDir == null) {
            logger.error("ConfigProvider does not support getConfigDir() - cannot use embedded mode")
            return false
        }

        val embedded = config.embeddedConfig
        if (!canStart(embedded)) {
            logger.error(
                "Embedded server needs TLS. Set server.tls.certificate and server.tls.key, or configure server.tls.acme."
            )
            return false
        }

        // Use absolute path to avoid issues with relative paths on Windows
        val configDirAbsolute = configDir.toAbsolutePath().toString()
        val builder = RuntimeConfigBuilder(configDirAbsolute)
        val runtimeConfig = builder.build(embedded, config.accessToken)
        accessToken = builder.resolvedAccessToken

        try {
            if (!Files.exists(configDir)) {
                Files.createDirectories(configDir)
                logger.debug("Created data directory: {}", configDir)
            }

            runtimeConfig.server?.tls?.certsPath?.let { certsPath ->
                Files.createDirectories(java.nio.file.Paths.get(certsPath))
            }

            runtimeConfig.audio?.filePath?.let { audioPath ->
                Files.createDirectories(java.nio.file.Paths.get(audioPath))
                logger.info("Audio assets directory: {}", audioPath)
            }
        } catch (e: Exception) {
            logger.error("Failed to create data directories {}: {}", configDir, e.message)
            return false
        }

        try {
            BvcNative.load()
            logger.info("Native library version: {}", BvcNative.getVersion())
        } catch (e: Exception) {
            logger.error("Failed to load native library: {}", e.message)
            return false
        }

        val configJson = builder.toJson(runtimeConfig)
        logger.debug("Creating server with config: {}", configJson)

        val serverHandle = BvcNative.createServer(configJson)
        if (serverHandle == null) {
            logger.error("Failed to create BVC server: {}", BvcNative.getLastError())
            return false
        }
        handle = serverHandle

        effectiveConfig = BvcNative.configEffective(serverHandle)?.let { json ->
            GSON.fromJson(json, EmbeddedServerConfig::class.java)
        }

        // Start server in dedicated thread (Java owns the thread)
        serverThread = Thread({
            logger.info("BVC server thread starting...")
            val result = BvcNative.startServer(serverHandle)
            if (result != 0) {
                logger.error("BVC server exited with error: {} - {}", result, BvcNative.getLastError())
            } else {
                logger.info("BVC server thread exited cleanly")
            }
        }, "BVC-Server")
        serverThread?.start()

        // Brief wait for startup
        Thread.sleep(100)

        logger.info(
            "Embedded BVC server started (HTTP:{}, QUIC:{})",
            effectiveConfig?.server?.port,
            effectiveConfig?.server?.quicPort
        )
        return true
    }

    /**
     * The configuration the server resolved, available once it has been created.
     *
     * Serde defaults and `BVC_*` overrides are applied by the server, so this is
     * the only place a caller can read what an unset key actually became.
     */
    fun effectiveConfig(): EmbeddedServerConfig? = effectiveConfig

    /**
     * Get the server handle for direct FFI calls.
     * @return the handle, or null if server not started
     */
    @Synchronized
    fun getHandle(): Pointer? = handle

    /**
     * Update player positions directly via FFI (bypasses HTTP).
     * Synchronized to prevent race with stop().
     * @param gameDataJson JSON string with game data
     * @return true on success
     */
    @Synchronized
    fun updatePositions(gameDataJson: String): Boolean {
        val h = handle ?: return false
        return BvcNative.updatePositions(h, gameDataJson) == 0
    }

    /**
     * Start audio playback via FFI.
     * @param playJson JSON string matching AudioPlayRequest structure
     * @return JSON string with AudioEventResponse on success, null on failure
     */
    @Synchronized
    fun audioPlay(playJson: String): String? {
        val h = handle ?: return null
        return BvcNative.audioPlay(h, playJson)
    }

    /**
     * Stop audio playback via FFI.
     * @param eventId Event ID to stop
     * @return true on success
     */
    @Synchronized
    fun audioStop(eventId: String): Boolean {
        val h = handle ?: return false
        return BvcNative.audioStop(h, eventId) == 0
    }

    /**
     * Submit an in-game control action via FFI.
     * @param json JSON string matching the common ClientAction structure
     * @return the outcome; groupCode carries the share code after a successful CreateGroup
     */
    @Synchronized
    fun clientAction(json: String): ControlSendResult {
        val h = handle ?: return ControlSendResult(false)
        return BvcNative.clientAction(h, json)
    }

    /**
     * Stop the embedded BVC server gracefully.
     * Synchronized to prevent double-free race condition.
     */
    @Synchronized
    fun stop() {
        val h = handle ?: return  // Early return if already stopped
        handle = null  // Clear immediately to prevent races
        effectiveConfig = null

        logger.info("Stopping embedded BVC server...")
        BvcNative.stopServer(h)

        // Wait for thread to finish
        val thread = serverThread
        serverThread = null
        try {
            thread?.join(5000)
            if (thread?.isAlive == true) {
                logger.warn("BVC server thread did not stop gracefully within 5 seconds")
            }
        } catch (e: InterruptedException) {
            Thread.currentThread().interrupt()  // Restore interrupt flag
            logger.warn("Interrupted while waiting for server thread")
        }

        BvcNative.destroyServer(h)
        logger.info("Embedded BVC server stopped")
    }

    @Synchronized
    override fun chatRegister(helloJson: String): Boolean {
        val h = handle ?: return false
        return BvcNative.chatRegister(h, helloJson)
    }

    @Synchronized
    override fun chatReport(chatJson: String): Boolean {
        val h = handle ?: return false
        return BvcNative.chatReport(h, chatJson)
    }

    /**
     * The embedded server's own peerlink, for a bridge running beside it.
     *
     * Minted from the live endpoint, so it carries a loopback address the bridge can
     * dial. Null when the server declares no peers and binds no peer endpoint.
     */
    @Synchronized
    fun serverPeerlink(): String? {
        val h = handle ?: return null
        return BvcNative.relayPeerlink(h)
    }

    /**
     * Whether a player holds a live voice connection to this embedded server.
     *
     * The SVC bridge asks so it can leave those players out of its injection: one
     * running both Simple Voice Chat and the BVC desktop client would otherwise hear
     * every remote speaker twice.
     */
    @Synchronized
    fun hasLiveClient(identity: String): Boolean {
        val h = handle ?: return false
        return BvcNative.hasLiveClient(h, identity)
    }

    /** Reports whether this host could fetch and write a native library. */
    @Synchronized
    fun hostCapability(reportJson: String): Boolean {
        val h = handle ?: return false
        return BvcNative.hostCapability(h, reportJson)
    }

    @Synchronized
    override fun chatDrain(): String? {
        val h = handle ?: return null
        return BvcNative.chatDrain(h)
    }

    @Synchronized
    override fun chatUnregister(): Boolean {
        val h = handle ?: return false
        return BvcNative.chatUnregister(h)
    }

    /**
     * Where the embedded server's HTTP listener can be reached, for the chat channel.
     *
     * The name comes from the TLS names rather than an address, because the listener always
     * serves TLS — `get_rocket_config` refuses to start without a certificate — and a
     * certificate is issued for a name. Dialling `127.0.0.1` would present an address the
     * certificate does not cover and fail verification.
     *
     * Whether that name resolves to this machine is a DNS question rather than a code one.
     * It normally does; a hosts entry makes it literally loopback.
     *
     * The values come from the server's resolved configuration, so a name or port the
     * operator never set is the one the server actually chose.
     */
    fun chatEndpoint(): String? {
        val server = effectiveConfig?.server ?: return null
        val host = server.tls?.names?.firstOrNull()?.takeIf { it.isNotBlank() } ?: return null
        val port = server.port ?: return null
        return "https://$host:$port"
    }
}
