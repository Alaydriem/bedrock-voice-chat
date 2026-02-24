package com.alaydriem.bedrockvoicechat.fabric.audio

import net.minecraft.block.BlockState
import net.minecraft.block.entity.BlockEntity
import net.minecraft.item.ItemStack
import net.minecraft.storage.ReadView
import net.minecraft.storage.WriteView
import net.minecraft.util.math.BlockPos

/**
 * Block entity for the BVC audio player block.
 * Stores the inserted audio disc with persistence via ReadView/WriteView.
 */
class AudioPlayerBlockEntity(
    pos: BlockPos,
    state: BlockState
) : BlockEntity(FabricAudioRegistry.AUDIO_PLAYER_BLOCK_ENTITY_TYPE, pos, state) {

    private var disc: ItemStack = ItemStack.EMPTY

    fun hasDisc(): Boolean = !disc.isEmpty

    fun insertDisc(item: ItemStack) {
        disc = item.copyWithCount(1)
        markDirty()
    }

    fun removeDisc(): ItemStack? {
        if (disc.isEmpty) return null
        val removed = disc.copy()
        disc = ItemStack.EMPTY
        markDirty()
        return removed
    }

    fun getDisc(): ItemStack = disc

    override fun writeData(view: WriteView) {
        super.writeData(view)
        if (!disc.isEmpty) {
            view.put("bvc_disc", ItemStack.CODEC, disc)
        }
    }

    override fun readData(view: ReadView) {
        super.readData(view)
        disc = view.read("bvc_disc", ItemStack.CODEC).orElse(ItemStack.EMPTY)
    }
}
