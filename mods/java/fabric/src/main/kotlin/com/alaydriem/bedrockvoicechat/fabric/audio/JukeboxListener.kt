package com.alaydriem.bedrockvoicechat.fabric.audio

import net.fabricmc.fabric.api.event.lifecycle.v1.ServerTickEvents
import net.fabricmc.fabric.api.event.player.PlayerBlockBreakEvents
import net.fabricmc.fabric.api.event.player.UseBlockCallback
import net.minecraft.core.BlockPos
import net.minecraft.core.component.DataComponents
import net.minecraft.core.particles.ParticleTypes
import net.minecraft.network.chat.Component
import net.minecraft.server.level.ServerLevel
import net.minecraft.server.level.ServerPlayer
import net.minecraft.world.InteractionHand
import net.minecraft.world.InteractionResult
import net.minecraft.world.item.ItemStack
import net.minecraft.world.item.Items
import net.minecraft.nbt.CompoundTag
import net.minecraft.world.item.component.CustomData
import net.minecraft.world.level.Level
import net.minecraft.world.level.block.Block
import net.minecraft.world.level.block.Blocks
import net.minecraft.world.level.block.JukeboxBlock
import net.minecraft.world.level.block.entity.JukeboxBlockEntity

class JukeboxListener(
    private val audioPlayerManager: FabricAudioPlayerManager,
    private val worldUuidResolver: (ServerLevel) -> String
) {
    private data class ActiveJukebox(val pos: BlockPos, val ejectTick: Long)

    private val activeJukeboxes = mutableMapOf<String, ActiveJukebox>()
    private var particleTickCounter = 0

    init {
        instance = this
    }

    fun register() {
        UseBlockCallback.EVENT.register { player, world, hand, hitResult ->
            if (world.isClientSide) return@register InteractionResult.PASS
            if (hand != InteractionHand.MAIN_HAND) return@register InteractionResult.PASS

            val pos = hitResult.blockPos
            val state = world.getBlockState(pos)
            if (state.block != Blocks.JUKEBOX) return@register InteractionResult.PASS

            val jukebox = world.getBlockEntity(pos) as? JukeboxBlockEntity
                ?: return@register InteractionResult.PASS

            val heldItem = player.getItemInHand(hand)

            if (heldItem.item == Items.MUSIC_DISC_5 && isBvcDisc(heldItem)) {
                if (!jukebox.theItem.isEmpty) return@register InteractionResult.PASS

                // Strip JUKEBOX_PLAYABLE before insertion to prevent vanilla audio.
                // The mixin handles BVC playback start, block state, and auto-eject.
                val disc = heldItem.copyWithCount(1)
                heldItem.shrink(1)
                disc.remove(DataComponents.JUKEBOX_PLAYABLE)
                jukebox.setTheItem(disc)

                (player as? ServerPlayer)?.sendSystemMessage(Component.empty(), true)

                InteractionResult.SUCCESS
            } else if (heldItem.isEmpty) {
                val record = jukebox.theItem
                if (record.isEmpty || !isBvcDisc(record)) return@register InteractionResult.PASS

                val worldUuid = worldUuidResolver(world as ServerLevel)
                val key = audioPlayerManager.locationKey(worldUuid, pos.x, pos.y, pos.z)
                audioPlayerManager.stopPlayback(key)
                activeJukeboxes.remove(key)
                ejectDisc(world as ServerLevel, pos, jukebox)

                InteractionResult.SUCCESS
            } else {
                InteractionResult.PASS
            }
        }

        PlayerBlockBreakEvents.BEFORE.register { world, player, pos, state, blockEntity ->
            if (state.block != Blocks.JUKEBOX) return@register true
            val jukebox = blockEntity as? JukeboxBlockEntity ?: return@register true
            if (jukebox.theItem.isEmpty || !isBvcDisc(jukebox.theItem)) return@register true

            restoreJukeboxPlayable(jukebox.theItem)

            val key = audioPlayerManager.locationKey(
                worldUuidResolver(world as ServerLevel), pos.x, pos.y, pos.z
            )
            audioPlayerManager.stopPlayback(key)
            activeJukeboxes.remove(key)
            true
        }

        ServerTickEvents.END_SERVER_TICK.register { server ->
            val currentTick = server.tickCount.toLong()

            val toEject = mutableListOf<String>()
            for ((key, active) in activeJukeboxes) {
                if (currentTick >= active.ejectTick) {
                    toEject.add(key)
                }
            }
            for (key in toEject) {
                val active = activeJukeboxes.remove(key) ?: continue
                audioPlayerManager.stopPlayback(key)
                for (world in server.allLevels) {
                    if (world.getBlockState(active.pos).block == Blocks.JUKEBOX) {
                        val jukebox = world.getBlockEntity(active.pos) as? JukeboxBlockEntity ?: continue
                        ejectDisc(world, active.pos, jukebox)
                        break
                    }
                }
            }

            particleTickCounter++
            if (particleTickCounter < 20) return@register
            particleTickCounter = 0

            for ((key, active) in activeJukeboxes) {
                if (!audioPlayerManager.hasActivePlayback(key)) continue
                for (world in server.allLevels) {
                    if (world.getBlockState(active.pos).block == Blocks.JUKEBOX) {
                        world.sendParticles(
                            ParticleTypes.NOTE,
                            active.pos.x + 0.5, active.pos.y + 1.2, active.pos.z + 0.5,
                            1, 0.3, 0.0, 0.3, 0.0
                        )
                        break
                    }
                }
            }
        }
    }

    companion object {
        private const val BVC_DISC_TAG = "bvc_disc"
        private const val AUDIO_ID_TAG = "audio_id"

        @Volatile
        var instance: JukeboxListener? = null
            private set

        @JvmStatic
        fun onHopperInsert(jukebox: JukeboxBlockEntity, stack: ItemStack) {
            val listener = instance ?: return
            val world = jukebox.level as? ServerLevel ?: return
            val pos = jukebox.blockPos

            stack.remove(DataComponents.JUKEBOX_PLAYABLE)
            jukebox.songPlayer.stop(world, world.getBlockState(pos))

            val audioId = getAudioId(stack) ?: return
            val dimensionId = getDimensionId(world)
            val worldUuid = listener.worldUuidResolver(world)
            val key = listener.audioPlayerManager.locationKey(worldUuid, pos.x, pos.y, pos.z)

            if (listener.audioPlayerManager.hasActivePlayback(key)) return

            world.setBlockAndUpdate(pos, world.getBlockState(pos).setValue(JukeboxBlock.HAS_RECORD, true))

            listener.audioPlayerManager.startPlayback(
                audioId, dimensionId,
                pos.x.toDouble(), pos.y.toDouble(), pos.z.toDouble(),
                worldUuid
            ) { durationMs ->
                val ejectTick = if (durationMs > 0) {
                    world.server?.let { it.tickCount + (durationMs / 50L) } ?: Long.MAX_VALUE
                } else Long.MAX_VALUE
                listener.activeJukeboxes[key] = ActiveJukebox(pos.immutable(), ejectTick)
            }
        }

        fun getDimensionId(world: Level): String {
            val path = world.dimension().identifier().path
            return when (path) {
                "the_nether" -> "nether"
                else -> path
            }
        }

        fun isBvcDisc(stack: ItemStack): Boolean {
            if (stack.item != Items.MUSIC_DISC_5) return false
            val nbt = stack.get(DataComponents.CUSTOM_DATA) ?: return false
            return nbt.copyTag().getBoolean(BVC_DISC_TAG).orElse(false)
        }

        fun getAudioId(stack: ItemStack): String? {
            val nbt = stack.get(DataComponents.CUSTOM_DATA) ?: return null
            return nbt.copyTag().getString(AUDIO_ID_TAG).orElse(null)
        }

        fun createBvcDisc(audioId: String): ItemStack {
            val disc = ItemStack(Items.MUSIC_DISC_5)
            val tag = CompoundTag()
            tag.putBoolean(BVC_DISC_TAG, true)
            tag.putString(AUDIO_ID_TAG, audioId)
            CustomData.set(DataComponents.CUSTOM_DATA, disc, tag)
            return disc
        }

        private fun ejectDisc(world: ServerLevel, pos: BlockPos, jukebox: JukeboxBlockEntity) {
            val disc = jukebox.theItem
            if (disc.isEmpty) return
            restoreJukeboxPlayable(disc)
            jukebox.setTheItem(ItemStack.EMPTY)
            jukebox.setChanged()
            world.setBlockAndUpdate(pos, world.getBlockState(pos).setValue(JukeboxBlock.HAS_RECORD, false))
            Block.popResource(world, pos.above(), disc)
        }

        private fun restoreJukeboxPlayable(disc: ItemStack) {
            if (disc.has(DataComponents.JUKEBOX_PLAYABLE)) return
            val defaultPlayable = ItemStack(Items.MUSIC_DISC_5).get(DataComponents.JUKEBOX_PLAYABLE)
            if (defaultPlayable != null) {
                disc.set(DataComponents.JUKEBOX_PLAYABLE, defaultPlayable)
            }
        }
    }
}
