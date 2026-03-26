package com.alaydriem.bedrockvoicechat.fabric.audio

import com.alaydriem.bedrockvoicechat.audio.AudioEventSender
import com.alaydriem.bedrockvoicechat.audio.AudioPlayerManager
import com.alaydriem.bedrockvoicechat.audio.AudioPlayerState
import com.alaydriem.bedrockvoicechat.audio.dto.AudioEventResponse
import com.alaydriem.bedrockvoicechat.audio.dto.AudioPlayRequest
import com.alaydriem.bedrockvoicechat.audio.dto.GameAudioRequest
import com.alaydriem.bedrockvoicechat.dto.Coordinates
import com.google.gson.Gson
import org.slf4j.LoggerFactory
import java.util.concurrent.ConcurrentHashMap

class FabricAudioPlayerManager(
    private val audioEventSender: AudioEventSender
) : AudioPlayerManager {

    private val activePlaybacks = ConcurrentHashMap<String, AudioPlayerState>()
    private val gson = Gson()

    companion object {
        private val logger = LoggerFactory.getLogger("BVC AudioPlayerManager")
    }

    override fun locationKey(worldUuid: String, x: Int, y: Int, z: Int): String =
        "$worldUuid:$x:$y:$z"

    fun startPlayback(
        audioId: String,
        dimensionId: String,
        x: Double,
        y: Double,
        z: Double,
        worldUuid: String,
        onStarted: (durationMs: Long) -> Unit
    ) {
        val key = locationKey(worldUuid, x.toInt(), y.toInt(), z.toInt())
        logger.debug("Starting playback: audioId={}, dimension={}, pos=({},{},{}), worldUuid={}, key={}",
            audioId, dimensionId, x, y, z, worldUuid, key)
        val state = AudioPlayerState(
            audioId = audioId,
            eventId = null,
            dimensionId = dimensionId,
            x = x,
            y = y,
            z = z,
            worldUuid = worldUuid
        )
        activePlaybacks[key] = state

        val request = AudioPlayRequest(
            audioFileId = audioId,
            game = GameAudioRequest(
                game = "minecraft",
                coordinates = Coordinates(x, y, z),
                dimension = dimensionId,
                worldUuid = worldUuid
            )
        )
        val json = gson.toJson(request)

        audioEventSender.play(json) { responseJson ->
            if (responseJson != null) {
                try {
                    val response = gson.fromJson(responseJson, AudioEventResponse::class.java)
                    state.eventId = response.eventId
                    state.isPlaying = true
                    onStarted(response.durationMs)
                } catch (e: Exception) {
                    logger.warn("Failed to parse audio event response: {}", e.message)
                    activePlaybacks.remove(key)
                    onStarted(0L)
                }
            } else {
                logger.warn("Audio playback request failed for key: {}", key)
                activePlaybacks.remove(key)
                onStarted(0L)
            }
        }
    }

    override fun startPlayback(
        audioId: String,
        dimensionId: String,
        x: Double,
        y: Double,
        z: Double,
        worldUuid: String
    ) {
        startPlayback(audioId, dimensionId, x, y, z, worldUuid) {}
    }

    override fun stopPlayback(locationKey: String) {
        val state = activePlaybacks.remove(locationKey)
        if (state == null) {
            logger.warn("stopPlayback: no active playback for key={}", locationKey)
            return
        }
        val eventId = state.eventId
        if (eventId == null) {
            logger.warn("stopPlayback: eventId is null for key={}", locationKey)
            return
        }
        logger.debug("stopPlayback: stopping eventId={} for key={}", eventId, locationKey)
        audioEventSender.stop(eventId)
    }

    override fun hasActivePlayback(locationKey: String): Boolean =
        activePlaybacks.containsKey(locationKey)

    override fun shutdown() {
        activePlaybacks.forEach { (_, state) ->
            state.eventId?.let { audioEventSender.stop(it) }
        }
        activePlaybacks.clear()
    }
}
