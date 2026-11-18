package com.alaydriem.bedrockvoicechat.powernukkitx.audio

import org.powernukkitx.block.Block
import org.powernukkitx.block.BlockID
import org.powernukkitx.block.BlockJukebox
import org.powernukkitx.event.EventHandler
import org.powernukkitx.event.EventPriority
import org.powernukkitx.event.Listener
import org.powernukkitx.event.block.BlockBreakEvent
import org.powernukkitx.event.player.PlayerInteractEvent
import org.powernukkitx.item.Item

/**
 * Inserts and ejects BVC discs at a vanilla jukebox.
 *
 * A BVC disc is a vanilla music disc underneath, and `BlockJukebox.onActivate` would store
 * it and start that disc's own song. So every interaction that would insert a BVC disc is
 * cancelled and the disc is stored with `setRecordItem` alone. Vanilla discs are left to
 * PowerNukkitX.
 */
class PnxJukeboxListener(
    private val playback: PnxJukeboxPlayback,
    private val registry: PnxJukeboxRegistry
) : Listener {

    @EventHandler(priority = EventPriority.HIGHEST, ignoreCancelled = true)
    fun onPlayerInteract(event: PlayerInteractEvent) {
        val action = event.getAction()
        // A held right click activates the block too.
        if (action != PlayerInteractEvent.Action.RIGHT_CLICK_BLOCK &&
            action != PlayerInteractEvent.Action.RIGHT_HOLD_BLOCK
        ) return
        val block = event.getBlock() ?: return
        if (block.getId() != BlockID.JUKEBOX) return
        val location = locationOf(block) ?: return

        val player = event.getPlayer()
        val inventory = player.getInventory()
        val held = inventory.getItemInMainHand()
        val holdingBvcDisc = PnxDisc.isBvcDisc(held)
        val active = playback.isActive(location)

        if (!holdingBvcDisc && !active) return

        val jukebox = (block as? BlockJukebox)?.getOrCreateBlockEntity()

        if (active) {
            event.setCancelled()
            // A held click repeats while the button stays down; only a fresh click ejects,
            // so the click that inserted a disc does not take it straight back out.
            if (action != PlayerInteractEvent.Action.RIGHT_CLICK_BLOCK) return
            // Any item ejects, as PowerNukkitX does for a jukebox that holds a disc.
            jukebox?.dropItem()
            playback.stop(location)
            return
        }

        // A disc already inside is ejected by PowerNukkitX itself, which plays nothing.
        if (jukebox != null && !jukebox.getRecordItem().isNull()) return

        event.setCancelled()
        if (jukebox == null) return

        val disc = held.clone()
        disc.setCount(1)
        jukebox.setRecordItem(disc)
        jukebox.setDirty()
        registry.add(location)

        if (!player.isCreative()) {
            held.setCount(held.getCount() - 1)
            inventory.setItemInMainHand(if (held.getCount() <= 0) Item.AIR else held)
        }

        val audioId = PnxDisc.audioId(disc) ?: return
        playback.start(location, audioId)
    }

    // The jukebox's own break handling drops the disc.
    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    fun onBlockBreak(event: BlockBreakEvent) {
        val block = event.getBlock()
        if (block.getId() != BlockID.JUKEBOX) return
        val location = locationOf(block) ?: return
        playback.stop(location)
    }

    private fun locationOf(block: Block): PnxJukeboxLocation? {
        val level = block.getLevel() ?: return null
        return PnxJukeboxLocation(level, block.getFloorX(), block.getFloorY(), block.getFloorZ())
    }
}
