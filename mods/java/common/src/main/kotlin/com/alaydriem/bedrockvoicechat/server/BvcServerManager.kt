package com.alaydriem.bedrockvoicechat.server

import com.alaydriem.bedrockvoicechat.api.ConfigProvider
import com.alaydriem.bedrockvoicechat.config.EmbeddedConfig
import com.alaydriem.bedrockvoicechat.config.ModConfig
import com.alaydriem.bedrockvoicechat.native.BvcNative
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
) {
    companion object {
        private val logger = LoggerFactory.getLogger("BVC Server")
        private val GSON = Gson()
    }

    // @Volatile ensures visibility across threads - handle is set in start(), read in multiple places
    @Volatile
    private var handle: Pointer? = null

    // @Volatile for thread visibility - serverThread is set in start(), checked in isRunning/stop
    @Volatile
    private var serverThread: Thread? = null

    val isRunning: Boolean
        get() = handle != null && serverThread?.isAlive == true

    /**
     * Start the embedded BVC server.
     * @return true if started successfully, false otherwise
     */
    fun start(): Boolean {
        if (!config.useEmbeddedServer) {
            logger.debug("Embedded server mode not enabled")
            return false
        }

        val configDir = configProvider.getConfigDir()
        if (configDir == null) {
            logger.error("ConfigProvider does not support getConfigDir() - cannot use embedded mode")
            return false
        }

        // Validate TLS certificates are configured
        val embedded = config.embeddedConfig ?: EmbeddedConfig()
        if (!embedded.hasTlsCertificates()) {
            logger.error("Embedded server requires TLS certificates. Configure tls-certificate and tls-key in embedded-config.")
            return false
        }

        // Ensure data directories exist for embedded server
        try {
            if (!Files.exists(configDir)) {
                Files.createDirectories(configDir)
                logger.debug("Created data directory: {}", configDir)
            }

            // Create certificates directory (for QUIC mTLS CA)
            val certsDir = configDir.resolve("certificates")
            if (!Files.exists(certsDir)) {
                Files.createDirectories(certsDir)
                logger.debug("Created certificates directory: {}", certsDir)
            }

            // Create assets directory (for server assets)
            val assetsDir = configDir.resolve("assets")
            if (!Files.exists(assetsDir)) {
                Files.createDirectories(assetsDir)
                logger.debug("Created assets directory: {}", assetsDir)
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

        // Use absolute path to avoid issues with relative paths on Windows
        val runtimeConfig = buildRuntimeConfig(configDir.toAbsolutePath().toString())
        val configJson = GSON.toJson(runtimeConfig)

        logger.debug("Creating server with config: {}", configJson)

        val serverHandle = BvcNative.createServer(configJson)
        if (serverHandle == null) {
            logger.error("Failed to create BVC server: {}", BvcNative.getLastError())
            return false
        }
        handle = serverHandle

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

        logger.info("Embedded BVC server started (HTTP:{}, QUIC:{})", embedded.httpPort, embedded.quicPort)
        return true
    }

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
     * Stop the embedded BVC server gracefully.
     * Synchronized to prevent double-free race condition.
     */
    @Synchronized
    fun stop() {
        val h = handle ?: return  // Early return if already stopped
        handle = null  // Clear immediately to prevent races

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

    /**
     * Build the runtime configuration JSON for the native server.
     *
     * Two certificate systems:
     * 1. HTTPS TLS (certificate, key) - Third-party signed certs for Rocket HTTP server
     * 2. QUIC mTLS (certs_path) - Auto-generated CA for QUIC client authentication
     */
    private fun buildRuntimeConfig(configDirPath: String): Map<String, Any?> {
        val embedded = config.embeddedConfig ?: EmbeddedConfig()
        val certsPath = "$configDirPath/certificates"
        val assetsPath = "$configDirPath/assets"

        // Generate a random access token if not configured
        val accessToken = config.accessToken?.takeIf { it.isNotBlank() }
            ?: UUID.randomUUID().toString()

        return mapOf(
            "database" to mapOf(
                "scheme" to "sqlite3",
                "database" to "$configDirPath/bvc.sqlite3"
            ),
            "server" to mapOf(
                "listen" to "0.0.0.0",
                "port" to embedded.httpPort,
                "quic_port" to embedded.quicPort,
                "public_addr" to embedded.publicAddr,
                "assets_path" to assetsPath,
                "tls" to mapOf(
                    "certificate" to embedded.tlsCertificate,  // Third-party signed cert for HTTPS
                    "key" to embedded.tlsKey,                  // Private key for HTTPS
                    "so_reuse_port" to false,
                    "certs_path" to certsPath,                 // Auto-generated CA for QUIC mTLS
                    "names" to embedded.tlsNames,
                    "ips" to embedded.tlsIps
                ),
                "minecraft" to mapOf(
                    "access_token" to accessToken,
                    "client_id" to "a17f9693-f01f-4d1d-ad12-1f179478375d"
                )
            ),
            "log" to mapOf(
                "level" to embedded.logLevel,
                "out" to "stdout"
            ),
            "voice" to mapOf(
                "broadcast_range" to embedded.broadcastRange
            )
        )
    }
}
