package com.alaydriem.bedrockvoicechat.fabric.audio

import net.fabricmc.fabric.api.event.lifecycle.v1.ServerTickEvents
import net.fabricmc.fabric.api.event.player.PlayerBlockBreakEvents
import net.fabricmc.fabric.api.event.player.UseBlockCallback
import net.minecraft.block.Blocks
import net.minecraft.block.JukeboxBlock
import net.minecraft.block.entity.JukeboxBlockEntity
import net.minecraft.component.DataComponentTypes
import net.minecraft.component.type.NbtComponent
import net.minecraft.item.ItemStack
import net.minecraft.item.Items
import net.minecraft.particle.ParticleTypes
import net.minecraft.server.network.ServerPlayerEntity
import net.minecraft.server.world.ServerWorld
import net.minecraft.text.Text
import net.minecraft.util.ActionResult
import net.minecraft.util.Hand
import net.minecraft.util.ItemScatterer
import net.minecraft.util.math.BlockPos
import net.minecraft.world.World

class JukeboxListener(
    private val audioPlayerManager: FabricAudioPlayerManager,
    private val worldUuidResolver: (ServerWorld) -> String
) {
    private data class ActiveJukebox(val pos: BlockPos, val ejectTick: Long)

    private val activeJukeboxes = mutableMapOf<String, ActiveJukebox>()
    private var particleTickCounter = 0

    init {
        instance = this
    }

    fun register() {
        UseBlockCallback.EVENT.register { player, world, hand, hitResult ->
            if (world.isClient) return@register ActionResult.PASS
            if (hand != Hand.MAIN_HAND) return@register ActionResult.PASS

            val pos = hitResult.blockPos
            val state = world.getBlockState(pos)
            if (state.block != Blocks.JUKEBOX) return@register ActionResult.PASS

            val jukebox = world.getBlockEntity(pos) as? JukeboxBlockEntity
                ?: return@register ActionResult.PASS

            val heldItem = player.getStackInHand(hand)

            if (heldItem.item == Items.MUSIC_DISC_5 && isBvcDisc(heldItem)) {
                if (!jukebox.stack.isEmpty) return@register ActionResult.PASS

                val disc = heldItem.copyWithCount(1)
                heldItem.decrement(1)
                jukebox.setDisc(disc)
                disc.remove(DataComponentTypes.JUKEBOX_PLAYABLE)

                world.setBlockState(pos, state.with(JukeboxBlock.HAS_RECORD, true))

                val audioId = getAudioId(disc) ?: return@register ActionResult.SUCCESS
                val dimensionId = getDimensionId(world)
                val worldUuid = worldUuidResolver(world as ServerWorld)
                val key = audioPlayerManager.locationKey(worldUuid, pos.x, pos.y, pos.z)

                audioPlayerManager.startPlayback(
                    audioId, dimensionId,
                    pos.x.toDouble(), pos.y.toDouble(), pos.z.toDouble(),
                    worldUuid
                ) { durationMs ->
                    if (durationMs > 0) {
                        val ejectTick = world.server?.let { it.ticks + (durationMs / 50L) } ?: Long.MAX_VALUE
                        activeJukeboxes[key] = ActiveJukebox(pos.toImmutable(), ejectTick)
                    } else {
                        activeJukeboxes[key] = ActiveJukebox(pos.toImmutable(), Long.MAX_VALUE)
                    }
                }

                (player as? ServerPlayerEntity)?.sendMessage(Text.empty(), true)

                ActionResult.SUCCESS
            } else if (heldItem.isEmpty) {
                val record = jukebox.stack
                if (record.isEmpty || !isBvcDisc(record)) return@register ActionResult.PASS

                val worldUuid = worldUuidResolver(world as ServerWorld)
                val key = audioPlayerManager.locationKey(worldUuid, pos.x, pos.y, pos.z)
                audioPlayerManager.stopPlayback(key)
                activeJukeboxes.remove(key)
                ejectDisc(world as ServerWorld, pos, jukebox)

                ActionResult.SUCCESS
            } else {
                ActionResult.PASS
            }
        }

        PlayerBlockBreakEvents.BEFORE.register { world, player, pos, state, blockEntity ->
            if (state.block != Blocks.JUKEBOX) return@register true
            val jukebox = blockEntity as? JukeboxBlockEntity ?: return@register true
            if (jukebox.stack.isEmpty || !isBvcDisc(jukebox.stack)) return@register true

            restoreJukeboxPlayable(jukebox.stack)

            val key = audioPlayerManager.locationKey(
                worldUuidResolver(world as ServerWorld), pos.x, pos.y, pos.z
            )
            audioPlayerManager.stopPlayback(key)
            activeJukeboxes.remove(key)
            true
        }

        ServerTickEvents.END_SERVER_TICK.register { server ->
            val currentTick = server.ticks.toLong()

            // Auto-eject expired jukeboxes
            val toEject = mutableListOf<String>()
            for ((key, active) in activeJukeboxes) {
                if (currentTick >= active.ejectTick) {
                    toEject.add(key)
                }
            }
            for (key in toEject) {
                val active = activeJukeboxes.remove(key) ?: continue
                audioPlayerManager.stopPlayback(key)
                for (world in server.worlds) {
                    if (world.getBlockState(active.pos).block == Blocks.JUKEBOX) {
                        val jukebox = world.getBlockEntity(active.pos) as? JukeboxBlockEntity ?: continue
                        ejectDisc(world, active.pos, jukebox)
                        break
                    }
                }
            }

            // Spawn note particles every 20 ticks
            particleTickCounter++
            if (particleTickCounter < 20) return@register
            particleTickCounter = 0

            for ((key, active) in activeJukeboxes) {
                if (!audioPlayerManager.hasActivePlayback(key)) continue
                for (world in server.worlds) {
                    if (world.getBlockState(active.pos).block == Blocks.JUKEBOX) {
                        world.spawnParticles(
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
            val world = jukebox.world as? ServerWorld ?: return
            val pos = jukebox.pos

            stack.remove(DataComponentTypes.JUKEBOX_PLAYABLE)

            val audioId = getAudioId(stack) ?: return
            val dimensionId = getDimensionId(world)
            val worldUuid = listener.worldUuidResolver(world)
            val key = listener.audioPlayerManager.locationKey(worldUuid, pos.x, pos.y, pos.z)

            if (listener.audioPlayerManager.hasActivePlayback(key)) return

            world.setBlockState(pos, world.getBlockState(pos).with(JukeboxBlock.HAS_RECORD, true))

            listener.audioPlayerManager.startPlayback(
                audioId, dimensionId,
                pos.x.toDouble(), pos.y.toDouble(), pos.z.toDouble(),
                worldUuid
            ) { durationMs ->
                val ejectTick = if (durationMs > 0) {
                    world.server?.let { it.ticks + (durationMs / 50L) } ?: Long.MAX_VALUE
                } else Long.MAX_VALUE
                listener.activeJukeboxes[key] = ActiveJukebox(pos.toImmutable(), ejectTick)
            }
        }

        fun getDimensionId(world: World): String {
            val path = world.registryKey.value.path
            return when (path) {
                "the_nether" -> "nether"
                else -> path
            }
        }

        fun isBvcDisc(stack: ItemStack): Boolean {
            if (stack.item != Items.MUSIC_DISC_5) return false
            val nbt = stack.get(DataComponentTypes.CUSTOM_DATA) ?: return false
            return nbt.copyNbt().getBoolean(BVC_DISC_TAG).orElse(false)
        }

        fun getAudioId(stack: ItemStack): String? {
            val nbt = stack.get(DataComponentTypes.CUSTOM_DATA) ?: return null
            return nbt.copyNbt().getString(AUDIO_ID_TAG).orElse(null)
        }

        fun createBvcDisc(audioId: String): ItemStack {
            val disc = ItemStack(Items.MUSIC_DISC_5)
            NbtComponent.set(DataComponentTypes.CUSTOM_DATA, disc) { nbt ->
                nbt.putBoolean(BVC_DISC_TAG, true)
                nbt.putString(AUDIO_ID_TAG, audioId)
            }
            return disc
        }

        private fun ejectDisc(world: ServerWorld, pos: BlockPos, jukebox: JukeboxBlockEntity) {
            val disc = jukebox.stack
            if (disc.isEmpty) return
            restoreJukeboxPlayable(disc)
            jukebox.setStack(ItemStack.EMPTY)
            jukebox.markDirty()
            world.setBlockState(pos, world.getBlockState(pos).with(JukeboxBlock.HAS_RECORD, false))
            ItemScatterer.spawn(world, pos.x + 0.5, pos.y + 1.0, pos.z + 0.5, disc)
        }

        private fun restoreJukeboxPlayable(disc: ItemStack) {
            if (disc.contains(DataComponentTypes.JUKEBOX_PLAYABLE)) return
            val defaultPlayable = ItemStack(Items.MUSIC_DISC_5).get(DataComponentTypes.JUKEBOX_PLAYABLE)
            if (defaultPlayable != null) {
                disc.set(DataComponentTypes.JUKEBOX_PLAYABLE, defaultPlayable)
            }
        }
    }
}
