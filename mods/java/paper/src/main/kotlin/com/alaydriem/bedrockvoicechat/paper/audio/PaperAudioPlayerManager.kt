package com.alaydriem.bedrockvoicechat.paper.audio

import com.alaydriem.bedrockvoicechat.audio.AudioEventSender
import com.alaydriem.bedrockvoicechat.audio.AudioPlayerManager
import com.alaydriem.bedrockvoicechat.audio.AudioPlayerState
import com.alaydriem.bedrockvoicechat.audio.dto.AudioEventResponse
import com.alaydriem.bedrockvoicechat.audio.dto.AudioPlayRequest
import com.alaydriem.bedrockvoicechat.audio.dto.GameAudioRequest
import com.alaydriem.bedrockvoicechat.svc.RelayWorld
import com.alaydriem.bedrockvoicechat.dto.Coordinates
import com.google.gson.Gson
import org.bukkit.plugin.java.JavaPlugin
import org.bukkit.scheduler.BukkitTask
import java.util.concurrent.ConcurrentHashMap

class PaperAudioPlayerManager(
    private val audioEventSender: AudioEventSender,
    private val plugin: JavaPlugin,
    // Declared on every playback so the peer boundary can forward it. Absent, the
    // playback is audible to this server's own clients and to nobody bridged.
    private val relayWorld: RelayWorld? = null
) : AudioPlayerManager {

    private val activePlaybacks = ConcurrentHashMap<String, AudioPlayerState>()
    private val ejectTasks = HashMap<String, BukkitTask>()
    private val gson = Gson()

    override fun locationKey(worldUuid: String, x: Int, y: Int, z: Int): String = "$worldUuid:$x:$y:$z"

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
                relayWorldUuid = relayWorld?.id()
            )
        )
        val json = gson.toJson(request)

        audioEventSender.play(json) { responseJson ->
            if (responseJson != null) {
                val response = gson.fromJson(responseJson, AudioEventResponse::class.java)
                state.eventId = response.eventId
                state.isPlaying = true
                val ticks = (response.durationMs / 50L).coerceAtLeast(1L)
                plugin.server.scheduler.runTask(plugin, Runnable {
                    onStarted(response.durationMs)
                    val task = plugin.server.scheduler.runTaskLater(plugin, Runnable {
                        autoEject(key)
                    }, ticks)
                    ejectTasks[key] = task
                })
            } else {
                activePlaybacks.remove(key)
            }
        }
    }

    override fun stopPlayback(locationKey: String) {
        ejectTasks.remove(locationKey)?.cancel()
        val state = activePlaybacks.remove(locationKey)
        if (state == null) {
            plugin.logger.warning("stopPlayback: no active playback for key=$locationKey")
            return
        }
        val eventId = state.eventId
        if (eventId == null) {
            plugin.logger.warning("stopPlayback: eventId is null for key=$locationKey")
            return
        }
        plugin.logger.info("stopPlayback: stopping eventId=$eventId for key=$locationKey")
        audioEventSender.stop(eventId)
    }

    override fun hasActivePlayback(locationKey: String): Boolean = activePlaybacks.containsKey(locationKey)

    private fun autoEject(locationKey: String) {
        ejectTasks.remove(locationKey)
        val state = activePlaybacks.remove(locationKey) ?: return
        state.eventId?.let { audioEventSender.stop(it) }
        // Playback ended: stop audio and unlock. The disc stays in the jukebox (matching vanilla)
        // so a hopper below collects it via normal extraction, or the player right-clicks to eject.
    }

    override fun shutdown() {
        ejectTasks.values.forEach { it.cancel() }
        ejectTasks.clear()
        activePlaybacks.forEach { (_, state) ->
            state.eventId?.let { audioEventSender.stop(it) }
        }
        activePlaybacks.clear()
    }
}
