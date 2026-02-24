package com.alaydriem.bedrockvoicechat.audio

import com.alaydriem.bedrockvoicechat.dto.AudioPlayRequest
import com.alaydriem.bedrockvoicechat.dto.Coordinates
import com.alaydriem.bedrockvoicechat.native.AudioSender
import org.slf4j.LoggerFactory
import java.util.concurrent.ConcurrentHashMap

/**
 * State for an audio player block at a specific location.
 */
data class AudioPlayerState(
    val audioId: String,
    var eventId: String? = null,
    var isPlaying: Boolean = false,
    var lastTransitionTime: Long = 0
)

/**
 * Manages audio player blocks across the game world.
 * Keyed by "world:x:y:z" string for stable hash across world reloads.
 *
 * Provides debounce (1s cooldown) to handle rapid redstone transitions.
 * All network calls are dispatched asynchronously via AudioSender to avoid blocking the game thread.
 */
class AudioPlayerManager(
    private val audioSender: AudioSender
) {
    companion object {
        private val logger = LoggerFactory.getLogger("BVC Audio Player")
        private const val DEBOUNCE_MS = 1000L
    }

    private val players = ConcurrentHashMap<String, AudioPlayerState>()

    /**
     * Build a stable location key for the concurrent map.
     */
    fun locationKey(worldUuid: String, x: Int, y: Int, z: Int): String {
        return "$worldUuid:$x:$y:$z"
    }

    /**
     * Insert a disc into an audio player block.
     */
    fun insertDisc(locationKey: String, audioId: String) {
        players[locationKey] = AudioPlayerState(audioId = audioId)
        logger.debug("Disc inserted at {}: {}", locationKey, audioId)
    }

    /**
     * Remove a disc from an audio player block.
     * Stops playback if currently playing.
     */
    fun removeDisc(locationKey: String) {
        val state = players.remove(locationKey) ?: return
        if (state.isPlaying && state.eventId != null) {
            stopPlayback(state)
        }
        logger.debug("Disc removed from {}", locationKey)
    }

    /**
     * Update the power state of an audio player block.
     * Powered = play, unpowered = stop. Subject to debounce.
     */
    fun updatePowerState(
        locationKey: String,
        powered: Boolean,
        coordinates: Coordinates,
        dimension: String,
        worldUuid: String
    ) {
        val state = players[locationKey] ?: return
        val now = System.currentTimeMillis()

        // Debounce rapid transitions
        if (now - state.lastTransitionTime < DEBOUNCE_MS) {
            return
        }
        state.lastTransitionTime = now

        if (powered && !state.isPlaying) {
            startPlayback(state, coordinates, dimension, worldUuid)
        } else if (!powered && state.isPlaying) {
            stopPlayback(state)
        }
    }

    /**
     * Handle block destruction. Stops playback and removes state.
     */
    fun onBlockDestroyed(locationKey: String) {
        val state = players.remove(locationKey) ?: return
        if (state.isPlaying && state.eventId != null) {
            stopPlayback(state)
        }
        logger.debug("Audio player block destroyed at {}", locationKey)
    }

    /**
     * Stop all active playbacks (e.g., on server shutdown).
     */
    fun stopAll() {
        players.values.filter { it.isPlaying && it.eventId != null }.forEach { state ->
            stopPlayback(state)
        }
        players.clear()
    }

    /**
     * Check if a disc is inserted at the given location.
     */
    fun hasDisc(locationKey: String): Boolean {
        return players.containsKey(locationKey)
    }

    /**
     * Get the audio ID of the disc at the given location, or null.
     */
    fun getAudioId(locationKey: String): String? {
        return players[locationKey]?.audioId
    }

    /**
     * Get the event ID for an active playback, or null.
     */
    fun getEventId(locationKey: String): String? {
        return players[locationKey]?.eventId
    }

    private fun startPlayback(
        state: AudioPlayerState,
        coordinates: Coordinates,
        dimension: String,
        worldUuid: String
    ) {
        val request = AudioPlayRequest(
            audio_file_id = state.audioId,
            coordinates = coordinates,
            dimension = dimension,
            world_uuid = worldUuid
        )

        state.isPlaying = true

        audioSender.playAsync(request).thenAccept { response ->
            if (response != null) {
                state.eventId = response.event_id
                logger.info("Audio playback started: {} (duration: {}ms)", response.event_id, response.duration_ms)
            } else {
                state.isPlaying = false
                logger.warn("Failed to start audio playback for {}", state.audioId)
            }
        }.exceptionally { ex ->
            state.isPlaying = false
            logger.error("Error starting audio playback: {}", ex.message)
            null
        }
    }

    private fun stopPlayback(state: AudioPlayerState) {
        val eventId = state.eventId ?: return
        state.isPlaying = false
        state.eventId = null

        audioSender.stopAsync(eventId).thenAccept { success ->
            if (success) {
                logger.info("Audio playback stopped: {}", eventId)
            } else {
                logger.warn("Failed to stop audio playback: {}", eventId)
            }
        }.exceptionally { ex ->
            logger.error("Error stopping audio playback: {}", ex.message)
            null
        }
    }
}
