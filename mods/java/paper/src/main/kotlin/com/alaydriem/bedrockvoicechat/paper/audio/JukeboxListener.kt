package com.alaydriem.bedrockvoicechat.paper.audio

import io.papermc.paper.datacomponent.DataComponentTypes
import net.kyori.adventure.text.Component
import org.bukkit.Material
import org.bukkit.NamespacedKey
import org.bukkit.World
import org.bukkit.block.Block
import org.bukkit.block.Jukebox
import org.bukkit.entity.Item
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

@Suppress("UnstableApiUsage")
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

        val item = event.item
        val isHoldingBvcDisc = item != null && item.type == Material.MUSIC_DISC_5 && isBvcDisc(item)

        val key = audioPlayerManager.locationKey(
            block.world.uid.toString(), block.x, block.y, block.z
        )
        val hasActiveBvcPlayback = audioPlayerManager.hasActivePlayback(key)

        if (!isHoldingBvcDisc && !hasActiveBvcPlayback) return

        val jukebox = block.state as? Jukebox ?: return

        if (hasActiveBvcPlayback) {
            if (item == null || item.type == Material.AIR) {
                event.isCancelled = true
                ejectBvcDisc(block, jukebox)
            } else {
                scheduleEjectCleanup(block)
            }
            return
        }

        if (isHoldingBvcDisc && !jukebox.hasRecord()) {
            insertBvcDisc(event, block, jukebox, item)
        }
    }

    private fun insertBvcDisc(event: PlayerInteractEvent, block: Block, jukebox: Jukebox, item: ItemStack) {
        val key = audioPlayerManager.locationKey(
            block.world.uid.toString(), block.x, block.y, block.z
        )
        if (audioPlayerManager.hasActivePlayback(key)) return

        event.isCancelled = true

        val disc = item.clone()
        disc.amount = 1
        disc.unsetData(DataComponentTypes.JUKEBOX_PLAYABLE)
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
    }

    private fun ejectBvcDisc(block: Block, jukebox: Jukebox) {
        val disc = jukebox.record.clone()
        restoreJukeboxPlayable(disc)
        jukebox.setRecord(null)
        jukebox.update(true)
        block.world.dropItemNaturally(block.location.add(0.5, 1.0, 0.5), disc)

        val key = audioPlayerManager.locationKey(
            block.world.uid.toString(), block.x, block.y, block.z
        )
        if (audioPlayerManager.hasActivePlayback(key)) {
            audioPlayerManager.stopPlayback(key)
        }
    }

    private fun scheduleEjectCleanup(block: Block) {
        val worldUuid = block.world.uid.toString()
        val key = audioPlayerManager.locationKey(worldUuid, block.x, block.y, block.z)

        plugin.server.scheduler.runTaskLater(plugin, Runnable {
            val state = block.state as? Jukebox
            val stillHasBvc = state?.hasRecord() == true && isBvcDisc(state.record)
            if (stillHasBvc) return@Runnable

            if (audioPlayerManager.hasActivePlayback(key)) {
                audioPlayerManager.stopPlayback(key)
            }

            val center = block.location.add(0.5, 0.5, 0.5)
            for (entity in block.world.getNearbyEntities(center, 2.0, 2.0, 2.0)) {
                if (entity !is Item) continue
                val stack = entity.itemStack
                if (stack.type != Material.MUSIC_DISC_5) continue
                if (!isBvcDisc(stack)) continue
                stack.resetData(DataComponentTypes.JUKEBOX_PLAYABLE)
                entity.itemStack = stack
            }
        }, 1L)
    }

    @EventHandler
    fun onInventoryMoveItem(event: InventoryMoveItemEvent) {
        val sourceHolder = event.source.holder
        if (sourceHolder is Jukebox) {
            val sourceBlock = sourceHolder.block
            val sourceKey = audioPlayerManager.locationKey(
                sourceBlock.world.uid.toString(), sourceBlock.x, sourceBlock.y, sourceBlock.z
            )
            if (audioPlayerManager.hasActivePlayback(sourceKey)) {
                event.isCancelled = true
            } else {
                val moving = event.item
                if (moving.type == Material.MUSIC_DISC_5 && isBvcDisc(moving)) {
                    restoreJukeboxPlayable(moving)
                    event.item = moving
                }
            }
            return
        }

        val destination = event.destination
        val holder = destination.holder as? Jukebox ?: return
        val item = event.item
        if (item.type != Material.MUSIC_DISC_5 || !isBvcDisc(item)) return

        val block = holder.block
        val world = block.world
        val key = audioPlayerManager.locationKey(world.uid.toString(), block.x, block.y, block.z)

        val state = block.state as? Jukebox ?: return
        if (state.hasRecord() || audioPlayerManager.hasActivePlayback(key)) {
            event.isCancelled = true
            return
        }

        val audioId = getAudioId(item) ?: return

        // Take over the insertion. A BVC disc must keep JUKEBOX_PLAYABLE to satisfy the hopper's
        // canPlaceItem check, but letting vanilla insert it would start the disc's own song. So cancel
        // the vanilla move, place a no-playable copy ourselves, consume one disc from the hopper, and
        // drive the audio through BVC.
        event.isCancelled = true

        val disc = item.clone().apply { amount = 1 }
        disc.unsetData(DataComponentTypes.JUKEBOX_PLAYABLE)
        state.setRecord(disc)
        state.update(true)

        event.source.removeItem(item.clone().apply { amount = 1 })

        audioPlayerManager.startPlayback(
            audioId, getDimensionId(world),
            block.x.toDouble(), block.y.toDouble(), block.z.toDouble(),
            world.uid.toString()
        )
    }

    @EventHandler
    fun onBlockBreak(event: BlockBreakEvent) {
        val block = event.block
        if (block.type != Material.JUKEBOX) return

        val jukebox = block.state as? Jukebox
        if (jukebox != null && jukebox.hasRecord()) {
            val record = jukebox.record
            if (isBvcDisc(record)) {
                restoreJukeboxPlayable(record)
                jukebox.setRecord(record)
                jukebox.update(true)
            }
        }

        val key = audioPlayerManager.locationKey(
            block.world.uid.toString(), block.x, block.y, block.z
        )
        if (audioPlayerManager.hasActivePlayback(key)) {
            audioPlayerManager.stopPlayback(key)
        }
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
        @Suppress("UnstableApiUsage")
        fun restoreJukeboxPlayable(disc: ItemStack) {
            if (disc.type != Material.MUSIC_DISC_5) return
            disc.resetData(DataComponentTypes.JUKEBOX_PLAYABLE)
        }
    }
}
