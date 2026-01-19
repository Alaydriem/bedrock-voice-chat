package com.alaydriem.bedrockvoicechat.native

import com.alaydriem.bedrockvoicechat.dto.Payload
import com.alaydriem.bedrockvoicechat.network.HttpRequestHandler
import com.alaydriem.bedrockvoicechat.server.BvcServerManager
import com.google.gson.Gson
import org.slf4j.LoggerFactory

/**
 * Routes position updates to either FFI (embedded server) or HTTP (external server).
 *
 * In embedded mode, positions are sent directly via FFI to avoid HTTP overhead.
 * In external mode, positions are sent via HTTP to the remote BVC server.
 */
class PositionSender(
    private val httpHandler: HttpRequestHandler?,
    private val embeddedServer: BvcServerManager?
) {
    companion object {
        private val logger = LoggerFactory.getLogger("BVC Position")
        private val GSON = Gson()
    }

    /**
     * Send position payload to the appropriate destination.
     * - Embedded mode: Direct FFI call (bypasses HTTP completely)
     * - External mode: HTTP POST to remote server
     */
    fun send(payload: Payload) {
        when {
            // Embedded mode: send directly via FFI (no HTTP overhead)
            embeddedServer != null && embeddedServer.isRunning -> {
                val json = GSON.toJson(payload)
                if (!embeddedServer.updatePositions(json)) {
                    logger.warn("Failed to send positions via FFI")
                }
            }
            // External server mode: use HTTP
            httpHandler != null -> {
                httpHandler.sendAsync(payload)
            }
            else -> {
                logger.warn("No position sender available (neither embedded nor HTTP configured)")
            }
        }
    }

    /**
     * Check if a position sender is available and ready.
     */
    fun isAvailable(): Boolean {
        return (embeddedServer?.isRunning == true) || httpHandler != null
    }
}
