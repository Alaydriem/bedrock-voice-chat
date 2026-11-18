package com.alaydriem.bedrockvoicechat.powernukkitx.audio

import com.alaydriem.bedrockvoicechat.audio.JukeboxPlaybackService
import com.alaydriem.bedrockvoicechat.powernukkitx.PnxDimension
import org.powernukkitx.level.Level

/**
 * BVC playback at a PowerNukkitX jukebox, for the player and hopper paths alike.
 *
 * When a track ends the disc stays in the jukebox and the location is free again, so a
 * hopper below can collect it. Nothing in the jukebox changes at that point: the disc was
 * placed without PowerNukkitX's own playback, so there is no vanilla sound to stop.
 */
class PnxJukeboxPlayback(
    private val service: JukeboxPlaybackService,
    private val worldUuidOf: (Level) -> String
) {

    fun isActive(location: PnxJukeboxLocation): Boolean = service.hasActivePlayback(key(location))

    fun start(location: PnxJukeboxLocation, audioId: String) {
        service.startPlayback(
            audioId,
            PnxDimension.of(location.level).toApiString(),
            location.x.toDouble(), location.y.toDouble(), location.z.toDouble(),
            worldUuidOf(location.level)
        )
    }

    fun stop(location: PnxJukeboxLocation) {
        val key = key(location)
        if (service.hasActivePlayback(key)) {
            service.stopPlayback(key)
        }
    }

    private fun key(location: PnxJukeboxLocation): String =
        service.locationKey(worldUuidOf(location.level), location.x, location.y, location.z)
}
