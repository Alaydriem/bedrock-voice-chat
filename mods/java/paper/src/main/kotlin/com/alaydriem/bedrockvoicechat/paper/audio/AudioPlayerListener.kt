package com.alaydriem.bedrockvoicechat.paper.audio

import com.alaydriem.bedrockvoicechat.audio.AudioPlayerManager
import com.alaydriem.bedrockvoicechat.dto.Coordinates
import org.bukkit.Material
import org.bukkit.NamespacedKey
import org.bukkit.block.Jukebox
import org.bukkit.event.EventHandler
import org.bukkit.event.Listener
import org.bukkit.event.block.BlockBreakEvent
import org.bukkit.event.world.GenericGameEvent
import org.bukkit.inventory.ItemStack
import org.bukkit.persistence.PersistentDataType
import org.bukkit.plugin.Plugin
import org.slf4j.LoggerFactory

/**
 * Listens for jukebox events to trigger BVC audio playback.
 *
 * Paper approach: vanilla jukebox with BVC disc items.
 * - Disc insert (via jukebox block interaction) = play
 * - Disc eject = stop
 * - Block break = stop + drop disc
 *
 * BVC discs are identified by PersistentDataContainer markers.
 */
class AudioPlayerListener(
    private val plugin: Plugin,
    private val audioPlayerManager: AudioPlayerManager,
    private val worldUuidProvider: () -> String?
) : Listener {

    companion object {
        private val logger = LoggerFactory.getLogger("BVC Audio Listener")

        val KEY_IS_BVC_DISC = NamespacedKey("bvc", "is_bvc_disc")
        val KEY_AUDIO_ID = NamespacedKey("bvc", "audio_id")
        val KEY_EVENT_ID = NamespacedKey("bvc", "event_id")
    }

    /**
     * Handle generic game events for jukebox play/stop.
     * Paper fires JUKEBOX_PLAY_RECORD and JUKEBOX_STOP_PLAYING_RECORD game events.
     */
    @EventHandler
    fun onGameEvent(event: GenericGameEvent) {
        val gameEvent = event.event
        val block = event.location.block

        if (block.type != Material.JUKEBOX) return

        val jukebox = block.state as? Jukebox ?: return

        when (gameEvent) {
            org.bukkit.GameEvent.JUKEBOX_PLAY -> {
                handleJukeboxPlay(jukebox)
            }
            org.bukkit.GameEvent.JUKEBOX_STOP_PLAY -> {
                handleJukeboxStop(jukebox)
            }
        }
    }

    private fun handleJukeboxPlay(jukebox: Jukebox) {
        val record = jukebox.record
        if (!isBvcDisc(record)) return

        val audioId = record.itemMeta?.persistentDataContainer
            ?.get(KEY_AUDIO_ID, PersistentDataType.STRING) ?: return

        val loc = jukebox.location
        val worldUuid = worldUuidProvider() ?: return
        val dimension = getDimension(loc.world)
        val coordinates = Coordinates(loc.x, loc.y, loc.z)
        val locationKey = audioPlayerManager.locationKey(
            worldUuid, loc.blockX, loc.blockY, loc.blockZ
        )

        // Cancel vanilla music playback
        jukebox.stopPlaying()

        audioPlayerManager.insertDisc(locationKey, audioId)
        audioPlayerManager.updatePowerState(locationKey, true, coordinates, dimension, worldUuid)

        logger.info("BVC disc inserted at {} - audio: {}", locationKey, audioId)
    }

    private fun handleJukeboxStop(jukebox: Jukebox) {
        val loc = jukebox.location
        val worldUuid = worldUuidProvider() ?: return
        val locationKey = audioPlayerManager.locationKey(
            worldUuid, loc.blockX, loc.blockY, loc.blockZ
        )

        if (!audioPlayerManager.hasDisc(locationKey)) return

        audioPlayerManager.removeDisc(locationKey)
        logger.info("BVC disc ejected at {}", locationKey)
    }

    /**
     * Handle jukebox block break - stop playback and let disc drop naturally.
     */
    @EventHandler
    fun onBlockBreak(event: BlockBreakEvent) {
        if (event.block.type != Material.JUKEBOX) return

        val jukebox = event.block.state as? Jukebox ?: return
        val loc = jukebox.location
        val worldUuid = worldUuidProvider() ?: return
        val locationKey = audioPlayerManager.locationKey(
            worldUuid, loc.blockX, loc.blockY, loc.blockZ
        )

        if (audioPlayerManager.hasDisc(locationKey)) {
            audioPlayerManager.onBlockDestroyed(locationKey)
            logger.info("Jukebox broken at {} - stopped playback", locationKey)
        }
    }

    /**
     * Check if an item is a BVC audio disc.
     */
    fun isBvcDisc(item: ItemStack): Boolean {
        val meta = item.itemMeta ?: return false
        return meta.persistentDataContainer.has(KEY_IS_BVC_DISC, PersistentDataType.BYTE)
    }

    /**
     * Create a BVC audio disc item.
     */
    fun createBvcDisc(audioId: String, displayName: String): ItemStack {
        val disc = ItemStack(Material.MUSIC_DISC_5, 1)
        val meta = disc.itemMeta!!

        meta.persistentDataContainer.set(KEY_IS_BVC_DISC, PersistentDataType.BYTE, 1.toByte())
        meta.persistentDataContainer.set(KEY_AUDIO_ID, PersistentDataType.STRING, audioId)
        meta.setDisplayName(net.kyori.adventure.text.Component.text(displayName)
            .let { net.kyori.adventure.text.serializer.legacy.LegacyComponentSerializer.legacySection().serialize(it) })

        disc.itemMeta = meta
        return disc
    }

    private fun getDimension(world: org.bukkit.World?): String {
        return when (world?.environment) {
            org.bukkit.World.Environment.NORMAL -> "overworld"
            org.bukkit.World.Environment.NETHER -> "nether"
            org.bukkit.World.Environment.THE_END -> "the_end"
            else -> "overworld"
        }
    }
}
