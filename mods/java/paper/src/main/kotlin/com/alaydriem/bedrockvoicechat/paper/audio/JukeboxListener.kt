package com.alaydriem.bedrockvoicechat.paper.audio

import net.kyori.adventure.text.Component
import org.bukkit.Material
import org.bukkit.NamespacedKey
import org.bukkit.World
import org.bukkit.block.Jukebox
import org.bukkit.event.EventHandler
import org.bukkit.event.EventPriority
import org.bukkit.event.Listener
import org.bukkit.event.block.Action
import org.bukkit.event.block.BlockBreakEvent
import org.bukkit.event.inventory.InventoryMoveItemEvent
import org.bukkit.event.player.PlayerInteractEvent
import org.bukkit.inventory.EquipmentSlot
import org.bukkit.inventory.ItemStack
import org.bukkit.persistence.PersistentDataType
import org.bukkit.plugin.java.JavaPlugin

class JukeboxListener(
    private val audioPlayerManager: PaperAudioPlayerManager,
    private val plugin: JavaPlugin
) : Listener {

    private val isBvcDiscKey = NamespacedKey(plugin, "is_bvc_disc")
    private val audioIdKey = NamespacedKey(plugin, "audio_id")

    @EventHandler(priority = EventPriority.HIGHEST)
    fun onPlayerInteract(event: PlayerInteractEvent) {
        if (event.action != Action.RIGHT_CLICK_BLOCK) return
        if (event.hand != EquipmentSlot.HAND) return
        val block = event.clickedBlock ?: return
        if (block.type != Material.JUKEBOX) return

        val jukebox = block.state as? Jukebox ?: return
        val item = event.item

        if (item != null && item.type == Material.MUSIC_DISC_5 && isBvcDisc(item)) {
            if (jukebox.hasRecord()) return

            val key = audioPlayerManager.locationKey(
                block.world.uid.toString(), block.x, block.y, block.z
            )
            if (audioPlayerManager.hasActivePlayback(key)) return

            event.isCancelled = true

            val disc = item.clone()
            disc.amount = 1
            jukebox.setRecord(disc)
            jukebox.update()

            disc.editMeta { it.setJukeboxPlayable(null) }
            jukebox.setRecord(disc)
            jukebox.update(true)

            if (item.amount <= 1) {
                event.player.inventory.setItemInMainHand(null)
            } else {
                item.amount--
            }

            val audioId = getAudioId(disc) ?: return
            val world = block.world
            val worldUuid = world.uid.toString()
            val dimensionId = getDimensionId(world)

            audioPlayerManager.startPlayback(
                audioId, dimensionId,
                block.x.toDouble(), block.y.toDouble(), block.z.toDouble(),
                worldUuid
            ) { _ -> }

            event.player.sendActionBar(Component.empty())

        } else if (item == null || item.type == Material.AIR) {
            if (!jukebox.hasRecord()) return
            val record = jukebox.record
            if (!isBvcDisc(record)) return

            event.isCancelled = true

            val disc = jukebox.record.clone()
            restoreJukeboxPlayable(disc)
            jukebox.setRecord(null)
            jukebox.update(true)
            block.world.dropItemNaturally(block.location.add(0.5, 1.0, 0.5), disc)

            val key = audioPlayerManager.locationKey(
                block.world.uid.toString(), block.x, block.y, block.z
            )
            audioPlayerManager.stopPlayback(key)
        }
    }

    @EventHandler
    fun onInventoryMoveItem(event: InventoryMoveItemEvent) {
        val destination = event.destination
        val holder = destination.holder as? Jukebox ?: return
        val item = event.item
        if (item.type != Material.MUSIC_DISC_5 || !isBvcDisc(item)) return

        val block = holder.block
        val audioId = getAudioId(item) ?: return
        val world = block.world

        plugin.server.scheduler.runTaskLater(plugin, Runnable {
            val state = block.state as? Jukebox ?: return@Runnable
            if (!state.hasRecord()) return@Runnable
            if (!isBvcDisc(state.record)) return@Runnable

            val worldUuid = world.uid.toString()
            val dimensionId = getDimensionId(world)
            audioPlayerManager.startPlayback(
                audioId, dimensionId,
                block.x.toDouble(), block.y.toDouble(), block.z.toDouble(),
                worldUuid
            )
        }, 1L)
    }

    @EventHandler
    fun onBlockBreak(event: BlockBreakEvent) {
        val block = event.block
        if (block.type != Material.JUKEBOX) return
        val jukebox = block.state as? Jukebox ?: return
        if (!jukebox.hasRecord() || !isBvcDisc(jukebox.record)) return

        restoreJukeboxPlayable(jukebox.record)
        jukebox.update(true)

        val key = audioPlayerManager.locationKey(
            block.world.uid.toString(), block.x, block.y, block.z
        )
        audioPlayerManager.stopPlayback(key)
    }

    private fun isBvcDisc(item: ItemStack?): Boolean {
        if (item == null || item.type == Material.AIR) return false
        val meta = item.itemMeta ?: return false
        return meta.persistentDataContainer.has(isBvcDiscKey, PersistentDataType.BOOLEAN)
    }

    private fun getAudioId(item: ItemStack?): String? {
        if (item == null) return null
        val meta = item.itemMeta ?: return null
        return meta.persistentDataContainer.get(audioIdKey, PersistentDataType.STRING)
    }

    private fun getDimensionId(world: World): String {
        return when (world.environment) {
            World.Environment.NETHER -> "nether"
            World.Environment.THE_END -> "the_end"
            else -> "overworld"
        }
    }

    companion object {
        fun restoreJukeboxPlayable(disc: ItemStack) {
            if (disc.type != Material.MUSIC_DISC_5) return
            val reference = ItemStack(Material.MUSIC_DISC_5)
            val refMeta = reference.itemMeta ?: return
            if (!refMeta.hasJukeboxPlayable()) return
            disc.editMeta { it.setJukeboxPlayable(refMeta.jukeboxPlayable) }
        }
    }
}
