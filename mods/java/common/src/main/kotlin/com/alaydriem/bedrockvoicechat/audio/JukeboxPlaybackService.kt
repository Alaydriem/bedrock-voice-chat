package com.alaydriem.bedrockvoicechat.audio

import com.alaydriem.bedrockvoicechat.audio.dto.AudioEventResponse
import com.alaydriem.bedrockvoicechat.audio.dto.AudioPlayRequest
import com.alaydriem.bedrockvoicechat.audio.dto.GameAudioRequest
import com.alaydriem.bedrockvoicechat.dto.Coordinates
import com.google.gson.Gson
import org.slf4j.LoggerFactory
import java.util.IdentityHashMap
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.CopyOnWriteArrayList

/**
 * Plays BVC discs at jukebox locations and ends each playback when its track runs out.
 *
 * Every state change runs on the main thread through [scheduler], whatever thread the
 * server's response arrives on. The service tracks location keys only; the platform maps
 * a key back to its block for the work a track's end needs (unlock, restore the disc).
 *
 * @param relayWorldId the world this server declares to its peers, read per playback.
 *   Absent, the playback is audible to this server's own clients and to nobody bridged.
 */
class JukeboxPlaybackService(
    private val sender: AudioEventPort,
    private val scheduler: MainThreadScheduler,
    private val relayWorldId: () -> String?
) : AudioPlayerManager {

    companion object {
        private val logger = LoggerFactory.getLogger("BVC JukeboxPlayback")
    }

    private val activePlaybacks = ConcurrentHashMap<String, AudioPlayerState>()
    private val ejectTasks = ConcurrentHashMap<String, ScheduledTask>()
    private val endedListeners = CopyOnWriteArrayList<(String) -> Unit>()
    private val gson = Gson()

    private val lock = Any()

    @Volatile
    private var closed = false

    // Keyed by identity: AudioPlayerState is a data class whose hash changes as it fills in.
    // Guarded by lock.
    private val pendingEvents = IdentityHashMap<AudioPlayerState, String>()

    override fun locationKey(worldUuid: String, x: Int, y: Int, z: Int): String = "$worldUuid:$x:$y:$z"

    /**
     * Registers [listener] to receive the location key of every playback that reached the
     * end of its track. A playback ended by [stopPlayback] or [shutdown] is not reported.
     */
    fun onEnded(listener: (locationKey: String) -> Unit) {
        endedListeners.add(listener)
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

    /**
     * Starts [audioId] at the given position. [onStarted] runs on the main thread with the
     * track length, or with 0 when the server refused or answered with something unusable;
     * the location is then already free. It is not called when the playback was stopped
     * before the server answered.
     */
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
                worldUuid = worldUuid,
                relayWorldUuid = relayWorldId()
            )
        )

        sender.play(gson.toJson(request)) { responseJson ->
            val response = parse(responseJson)
            if (!admitResponse(state, response)) return@play
            scheduler.runNow {
                if (claimResponse(state, response)) {
                    onResponse(key, state, response, onStarted)
                }
            }
        }
    }

    // After shutdown the main thread may never run again (Paper drops tasks for a disabled
    // plugin, Fabric stops ticking), so a late event is stopped here on the callback thread.
    // Otherwise a started event is held in pendingEvents until the main thread claims it,
    // so that a shutdown in between can stop it instead.
    private fun admitResponse(state: AudioPlayerState, response: AudioEventResponse?): Boolean {
        if (response == null) return !closed
        synchronized(lock) {
            if (!closed) {
                pendingEvents[state] = response.eventId
                return true
            }
        }
        sender.stop(response.eventId)
        return false
    }

    private fun claimResponse(state: AudioPlayerState, response: AudioEventResponse?): Boolean {
        if (response == null) return !closed
        synchronized(lock) {
            return pendingEvents.remove(state) != null
        }
    }

    override fun stopPlayback(locationKey: String) {
        ejectTasks.remove(locationKey)?.cancel()
        val state = activePlaybacks.remove(locationKey)
        if (state == null) {
            logger.debug("stopPlayback: no active playback for key={}", locationKey)
            return
        }
        state.eventId?.let { sender.stop(it) }
    }

    override fun hasActivePlayback(locationKey: String): Boolean = activePlaybacks.containsKey(locationKey)

    override fun shutdown() {
        val unclaimed = synchronized(lock) {
            closed = true
            pendingEvents.values.toList().also { pendingEvents.clear() }
        }
        unclaimed.forEach { sender.stop(it) }
        ejectTasks.values.forEach { it.cancel() }
        ejectTasks.clear()
        activePlaybacks.values.forEach { state -> state.eventId?.let { sender.stop(it) } }
        activePlaybacks.clear()
    }

    private fun parse(responseJson: String?): AudioEventResponse? {
        if (responseJson == null) return null
        val response = try {
            gson.fromJson(responseJson, AudioEventResponse::class.java)
        } catch (e: Exception) {
            logger.warn("Unparseable audio event response: {}", e.message)
            return null
        }
        // Gson leaves a missing event_id null despite the non-null declaration.
        @Suppress("SENSELESS_COMPARISON")
        if (response == null || response.eventId == null) {
            logger.warn("Audio event response carries no event_id")
            return null
        }
        return response
    }

    private fun onResponse(
        key: String,
        state: AudioPlayerState,
        response: AudioEventResponse?,
        onStarted: (durationMs: Long) -> Unit
    ) {
        // Stopped (or replaced) before the server answered. An event started now has no
        // owner left to stop it, so stop it here.
        if (activePlaybacks[key] !== state) {
            response?.let { sender.stop(it.eventId) }
            return
        }

        if (response == null) {
            logger.warn("Audio playback request failed for key={}", key)
            activePlaybacks.remove(key, state)
            onStarted(0L)
            return
        }

        state.eventId = response.eventId
        state.isPlaying = true
        onStarted(response.durationMs)

        val ticks = (response.durationMs / 50L).coerceAtLeast(1L)
        ejectTasks[key] = scheduler.runLater(ticks) { autoEject(key, state) }
    }

    private fun autoEject(key: String, state: AudioPlayerState) {
        if (!activePlaybacks.remove(key, state)) return
        ejectTasks.remove(key)
        state.eventId?.let { sender.stop(it) }
        endedListeners.forEach { it(key) }
    }
}
