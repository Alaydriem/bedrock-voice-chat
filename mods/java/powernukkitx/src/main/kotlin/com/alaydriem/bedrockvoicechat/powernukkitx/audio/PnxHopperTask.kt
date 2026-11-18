package com.alaydriem.bedrockvoicechat.powernukkitx.audio

import org.powernukkitx.block.BlockHopper
import org.powernukkitx.blockentity.BlockEntityHopper
import org.powernukkitx.blockentity.BlockEntityJukebox
import org.powernukkitx.item.Item
import org.powernukkitx.math.BlockFace

/**
 * Moves BVC discs between hoppers and jukeboxes, which PowerNukkitX hoppers do not do.
 *
 * A hopper above or beside a jukebox, with its output facing it, feeds it a BVC disc. The
 * hopper directly below pulls a finished BVC disc out, so a hopper chain plays disc after
 * disc. Run on the main thread every [PERIOD_TICKS].
 */
class PnxHopperTask(
    private val registry: PnxJukeboxRegistry,
    private val playback: PnxJukeboxPlayback,
    private val rule: HopperJukeboxRule
) : Runnable {

    override fun run() {
        for (location in registry.all()) {
            visit(location)
        }
    }

    private fun visit(location: PnxJukeboxLocation) {
        val level = location.level
        if (!level.isChunkLoaded(location.chunkX, location.chunkZ)) return

        val jukebox = level.getBlockEntityIfLoaded(location.vector()) as? BlockEntityJukebox
        if (jukebox == null || jukebox.closed) {
            // Gone without a player breaking it: an explosion, a piston, a command.
            registry.remove(location)
            playback.stop(location)
            return
        }

        extract(location, jukebox)
        insert(location, jukebox)
    }

    private fun extract(location: PnxJukeboxLocation, jukebox: BlockEntityJukebox) {
        val hopper = hopperAt(location.offset(0, -1, 0)) ?: return
        val record = jukebox.getRecordItem()
        val inventory = hopper.getInventory()

        val pull = rule.shouldExtract(
            jukeboxHasDisc = !record.isNull(),
            discIsBvc = PnxDisc.isBvcDisc(record),
            playbackActive = playback.isActive(location),
            hopperBelowJukebox = true,
            hopperHasRoom = inventory.canAddItem(record)
        )
        if (!pull) return

        inventory.addItem(record.clone())
        jukebox.setRecordItem(Item.AIR)
        jukebox.setDirty()
    }

    private fun insert(location: PnxJukeboxLocation, jukebox: BlockEntityJukebox) {
        val jukeboxHasDisc = !jukebox.getRecordItem().isNull()
        val active = playback.isActive(location)

        for (side in FEEDING_SIDES) {
            val hopperLocation = location.offset(side.getXOffset(), side.getYOffset(), side.getZOffset())
            val hopper = hopperAt(hopperLocation) ?: continue
            val facing = (hopper.getLevelBlock(false) as? BlockHopper)?.getBlockFace() ?: continue
            val facesJukebox = hopperLocation.offset(facing.getXOffset(), facing.getYOffset(), facing.getZOffset()) == location

            val inventory = hopper.getInventory()
            val slot = (0 until inventory.getSize()).firstOrNull { PnxDisc.isBvcDisc(inventory.getItem(it)) }
                ?: continue

            val feed = rule.shouldInsert(
                jukeboxHasDisc = jukeboxHasDisc,
                playbackActive = active,
                itemIsBvcDisc = true,
                hopperFacesJukebox = facesJukebox
            )
            if (!feed) continue

            val stack = inventory.getItem(slot)
            val disc = stack.clone()
            disc.setCount(1)
            stack.setCount(stack.getCount() - 1)
            inventory.setItem(slot, if (stack.getCount() <= 0) Item.AIR else stack)

            jukebox.setRecordItem(disc)
            jukebox.setDirty()

            val audioId = PnxDisc.audioId(disc) ?: return
            playback.start(location, audioId)
            return
        }
    }

    // A hopper locked by redstone moves nothing, as in vanilla.
    private fun hopperAt(location: PnxJukeboxLocation): BlockEntityHopper? {
        val hopper = location.level.getBlockEntityIfLoaded(location.vector()) as? BlockEntityHopper ?: return null
        if (hopper.closed || hopper.isDisabled()) return null
        return hopper
    }

    companion object {
        // The vanilla hopper transfer cooldown.
        const val PERIOD_TICKS = 8

        // A hopper below a jukebox pulls from it and never feeds it.
        private val FEEDING_SIDES = listOf(BlockFace.UP, BlockFace.NORTH, BlockFace.SOUTH, BlockFace.EAST, BlockFace.WEST)
    }
}
