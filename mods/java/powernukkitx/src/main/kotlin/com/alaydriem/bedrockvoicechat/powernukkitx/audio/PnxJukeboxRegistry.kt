package com.alaydriem.bedrockvoicechat.powernukkitx.audio

import org.powernukkitx.Server
import org.powernukkitx.block.BlockID
import org.powernukkitx.blockentity.BlockEntityJukebox
import org.powernukkitx.event.EventHandler
import org.powernukkitx.event.EventPriority
import org.powernukkitx.event.Listener
import org.powernukkitx.event.block.BlockBreakEvent
import org.powernukkitx.event.block.BlockPlaceEvent
import org.powernukkitx.event.level.ChunkLoadEvent
import org.powernukkitx.event.level.ChunkUnloadEvent
import org.powernukkitx.event.level.LevelUnloadEvent
import org.powernukkitx.level.Level
import org.powernukkitx.level.format.IChunk
import java.util.concurrent.ConcurrentHashMap

/**
 * The jukeboxes in loaded chunks, which the hopper feed visits.
 *
 * PowerNukkitX has no hopper-to-jukebox transfer to hook, so the feed polls. Keeping the
 * known positions here limits that poll to jukeboxes instead of every block entity.
 * A jukebox placed by anything but a player is found when its chunk next loads.
 */
class PnxJukeboxRegistry : Listener {

    // Chunk load events can arrive off the main thread while the hopper task reads.
    private val locations: MutableSet<PnxJukeboxLocation> = ConcurrentHashMap.newKeySet()

    fun all(): List<PnxJukeboxLocation> = locations.toList()

    fun add(location: PnxJukeboxLocation) {
        locations.add(location)
    }

    fun remove(location: PnxJukeboxLocation) {
        locations.remove(location)
    }

    fun removeLevel(level: Level) {
        locations.removeIf { it.level === level }
    }

    fun seed(server: Server) {
        for (level in server.getLevels().values) {
            for (chunk in level.getChunks().values) {
                addJukeboxesIn(chunk)
            }
        }
    }

    @EventHandler(priority = EventPriority.MONITOR)
    fun onChunkLoad(event: ChunkLoadEvent) {
        addJukeboxesIn(event.getChunk())
    }

    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    fun onChunkUnload(event: ChunkUnloadEvent) {
        val chunk = event.getChunk()
        val level = chunk.getLevel() ?: return
        locations.removeIf { it.level === level && it.chunkX == chunk.getX() && it.chunkZ == chunk.getZ() }
    }

    // An unloaded level must not stay reachable through its jukeboxes.
    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    fun onLevelUnload(event: LevelUnloadEvent) {
        removeLevel(event.getLevel())
    }

    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    fun onBlockPlace(event: BlockPlaceEvent) {
        val block = event.getBlock()
        if (block.getId() != BlockID.JUKEBOX) return
        val level = block.getLevel() ?: return
        locations.add(PnxJukeboxLocation(level, block.getFloorX(), block.getFloorY(), block.getFloorZ()))
    }

    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    fun onBlockBreak(event: BlockBreakEvent) {
        val block = event.getBlock()
        if (block.getId() != BlockID.JUKEBOX) return
        val level = block.getLevel() ?: return
        locations.remove(PnxJukeboxLocation(level, block.getFloorX(), block.getFloorY(), block.getFloorZ()))
    }

    private fun addJukeboxesIn(chunk: IChunk) {
        val level = chunk.getLevel() ?: return
        for (blockEntity in chunk.getBlockEntities().values) {
            if (blockEntity !is BlockEntityJukebox) continue
            locations.add(
                PnxJukeboxLocation(level, blockEntity.getFloorX(), blockEntity.getFloorY(), blockEntity.getFloorZ())
            )
        }
    }
}
