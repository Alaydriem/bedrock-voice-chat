package com.alaydriem.bedrockvoicechat.native

import com.alaydriem.bedrockvoicechat.dto.AudioEventResponse
import com.alaydriem.bedrockvoicechat.dto.AudioPlayRequest
import com.alaydriem.bedrockvoicechat.network.HttpRequestHandler
import com.alaydriem.bedrockvoicechat.server.BvcServerManager
import com.google.gson.Gson
import org.slf4j.LoggerFactory
import java.util.concurrent.CompletableFuture

/**
 * Routes audio play/stop requests to either FFI (embedded server) or HTTP (external server).
 * Mirrors the PositionSender dual-mode pattern.
 *
 * All operations return CompletableFuture to avoid blocking the game server thread.
 */
class AudioSender(
    private val httpHandler: HttpRequestHandler?,
    private val embeddedServer: BvcServerManager?
) {
    companion object {
        private val logger = LoggerFactory.getLogger("BVC Audio")
        private val GSON = Gson()
    }

    /**
     * Send a play request asynchronously.
     * @return CompletableFuture with the event response (event_id + duration), or null on failure
     */
    fun playAsync(request: AudioPlayRequest): CompletableFuture<AudioEventResponse?> {
        return when {
            embeddedServer != null && embeddedServer.isRunning -> {
                CompletableFuture.supplyAsync {
                    val json = GSON.toJson(request)
                    val result = embeddedServer.audioPlay(json)
                    if (result != null) {
                        GSON.fromJson(result, AudioEventResponse::class.java)
                    } else {
                        logger.warn("Failed to play audio via FFI")
                        null
                    }
                }
            }
            httpHandler != null -> {
                httpHandler.playAudioAsync(request)
            }
            else -> {
                logger.warn("No audio sender available (neither embedded nor HTTP configured)")
                CompletableFuture.completedFuture(null)
            }
        }
    }

    /**
     * Send a stop request asynchronously.
     * @return CompletableFuture that completes with true on success
     */
    fun stopAsync(eventId: String): CompletableFuture<Boolean> {
        return when {
            embeddedServer != null && embeddedServer.isRunning -> {
                CompletableFuture.supplyAsync {
                    embeddedServer.audioStop(eventId)
                }
            }
            httpHandler != null -> {
                httpHandler.stopAudioAsync(eventId)
            }
            else -> {
                logger.warn("No audio sender available (neither embedded nor HTTP configured)")
                CompletableFuture.completedFuture(false)
            }
        }
    }

    /**
     * Check if an audio sender is available and ready.
     */
    fun isAvailable(): Boolean {
        return (embeddedServer?.isRunning == true) || httpHandler != null
    }
}
