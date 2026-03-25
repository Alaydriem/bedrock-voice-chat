package com.alaydriem.bedrockvoicechat.audio

import com.alaydriem.bedrockvoicechat.network.HttpRequestHandler
import com.alaydriem.bedrockvoicechat.server.BvcServerManager
import org.slf4j.LoggerFactory

/**
 * Routes audio play/stop events to either FFI (embedded server) or HTTP (external server).
 *
 * In embedded mode, audio events are sent directly via FFI to avoid HTTP overhead.
 * In external mode, audio events are sent via HTTP to the remote BVC server.
 */
class AudioEventSender(
    private val httpHandler: HttpRequestHandler?,
    private val embeddedServer: BvcServerManager?
) {
    companion object {
        private val logger = LoggerFactory.getLogger("BVC Audio")
    }

    /**
     * Start audio playback.
     * - Embedded mode: Direct FFI call, callback is called synchronously with result
     * - External mode: HTTP POST to remote server, callback is called from CompletableFuture thread
     *
     * @param playJson JSON string matching AudioPlayRequest structure
     * @param callback Receives event ID on success, null on failure
     */
    fun play(playJson: String, callback: (String?) -> Unit) {
        when {
            // Embedded mode: send directly via FFI
            embeddedServer != null && embeddedServer.isRunning -> {
                val result = embeddedServer.audioPlay(playJson)
                if (result != null) {
                    callback(result)
                } else {
                    logger.warn("Failed to start audio playback via FFI")
                    callback(null)
                }
            }
            // External server mode: use HTTP
            httpHandler != null -> {
                httpHandler.audioPlayAsync(playJson, callback)
            }
            else -> {
                logger.warn("No audio event sender available (neither embedded nor HTTP configured)")
                callback(null)
            }
        }
    }

    /**
     * Stop audio playback.
     * - Embedded mode: Direct FFI call
     * - External mode: HTTP DELETE to remote server
     *
     * @param eventId Event ID to stop
     */
    fun stop(eventId: String) {
        when {
            // Embedded mode: send directly via FFI
            embeddedServer != null && embeddedServer.isRunning -> {
                if (!embeddedServer.audioStop(eventId)) {
                    logger.warn("Failed to stop audio playback via FFI for event: {}", eventId)
                }
            }
            // External server mode: use HTTP
            httpHandler != null -> {
                httpHandler.audioStopAsync(eventId)
            }
            else -> {
                logger.warn("No audio event sender available (neither embedded nor HTTP configured)")
            }
        }
    }

    /**
     * Check if an audio event sender is available and ready.
     *
     * @return true if embedded server is running or HTTP handler is configured
     */
    fun isAvailable(): Boolean {
        return (embeddedServer?.isRunning == true) || httpHandler != null
    }
}
