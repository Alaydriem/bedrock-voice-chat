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
import net.minecraft.world.Containers
import net.minecraft.world.InteractionHand
import net.minecraft.world.InteractionResult
import net.minecraft.world.entity.item.ItemEntity
import net.minecraft.world.item.ItemStack
import net.minecraft.world.item.Items
import net.minecraft.world.item.component.CustomData
import net.minecraft.world.level.Level
import net.minecraft.world.level.block.Blocks
import net.minecraft.world.level.block.JukeboxBlock
import net.minecraft.world.level.block.entity.JukeboxBlockEntity
import net.minecraft.world.phys.AABB

class JukeboxListener(
    private val audioPlayerManager: FabricAudioPlayerManager,
    private val worldUuidResolver: (ServerLevel) -> String
) {
    private data class ActiveJukebox(val pos: BlockPos, val ejectTick: Long)

    private data class PendingEjectCheck(
        val world: ServerLevel,
        val pos: BlockPos,
        val key: String,
        val tickAt: Long
    )

    private val activeJukeboxes = mutableMapOf<String, ActiveJukebox>()
    private val pendingEjectChecks = mutableListOf<PendingEjectCheck>()
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

            val heldItem = player.getItemInHand(hand)
            val isHoldingBvcDisc = heldItem.item == Items.MUSIC_DISC_5 && isBvcDisc(heldItem)

            val serverWorld = world as ServerLevel
            val worldUuid = worldUuidResolver(serverWorld)
            val key = audioPlayerManager.locationKey(worldUuid, pos.x, pos.y, pos.z)
            val hasActiveBvcPlayback = audioPlayerManager.hasActivePlayback(key)

            if (!isHoldingBvcDisc && !hasActiveBvcPlayback) return@register InteractionResult.PASS

            val jukebox = world.getBlockEntity(pos) as? JukeboxBlockEntity
                ?: return@register InteractionResult.PASS

            if (hasActiveBvcPlayback) {
                if (heldItem.isEmpty) {
                    audioPlayerManager.stopPlayback(key)
                    activeJukeboxes.remove(key)
                    ejectDisc(serverWorld, pos, jukebox)
                    return@register InteractionResult.SUCCESS
                }

                val server = serverWorld.server ?: return@register InteractionResult.PASS
                pendingEjectChecks.add(
                    PendingEjectCheck(serverWorld, pos.immutable(), key, server.tickCount + 1L)
                )
                return@register InteractionResult.PASS
            }

            if (isHoldingBvcDisc && jukebox.theItem.isEmpty) {
                val disc = heldItem.copyWithCount(1)
                heldItem.shrink(1)
                jukebox.setSongItemWithoutPlaying(disc)
                disc.remove(DataComponents.JUKEBOX_PLAYABLE)

                world.setBlock(pos, state.setValue(JukeboxBlock.HAS_RECORD, true), 3)

                val audioId = getAudioId(disc) ?: return@register InteractionResult.SUCCESS
                val dimensionId = getDimensionId(world)

                audioPlayerManager.startPlayback(
                    audioId, dimensionId,
                    pos.x.toDouble(), pos.y.toDouble(), pos.z.toDouble(),
                    worldUuid
                ) { durationMs ->
                    if (durationMs > 0) {
                        val ejectTick = world.server?.let { it.tickCount + (durationMs / 50L) } ?: Long.MAX_VALUE
                        activeJukeboxes[key] = ActiveJukebox(pos.immutable(), ejectTick)
                    } else {
                        activeJukeboxes[key] = ActiveJukebox(pos.immutable(), Long.MAX_VALUE)
                    }
                }

                (player as? ServerPlayer)?.sendOverlayMessage(Component.empty())

                return@register InteractionResult.SUCCESS
            }

            InteractionResult.PASS
        }

        PlayerBlockBreakEvents.BEFORE.register { world, _, pos, state, blockEntity ->
            if (state.block != Blocks.JUKEBOX) return@register true

            val jukebox = blockEntity as? JukeboxBlockEntity
            if (jukebox != null && !jukebox.theItem.isEmpty && isBvcDisc(jukebox.theItem)) {
                restoreJukeboxPlayable(jukebox.theItem)
            }

            val key = audioPlayerManager.locationKey(
                worldUuidResolver(world as ServerLevel), pos.x, pos.y, pos.z
            )
            if (audioPlayerManager.hasActivePlayback(key)) {
                audioPlayerManager.stopPlayback(key)
            }
            activeJukeboxes.remove(key)
            true
        }

        ServerTickEvents.END_SERVER_TICK.register { server ->
            val currentTick = server.tickCount.toLong()

            if (pendingEjectChecks.isNotEmpty()) {
                val due = pendingEjectChecks.filter { it.tickAt <= currentTick }
                pendingEjectChecks.removeAll(due)
                for (check in due) {
                    processEjectCheck(check)
                }
            }

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
                        // Leave the disc in the jukebox (now unlocked) so a hopper below collects it,
                        // matching vanilla where a finished disc stays until pulled
                        if (!jukebox.theItem.isEmpty && isBvcDisc(jukebox.theItem)) {
                            restoreJukeboxPlayable(jukebox.theItem)
                            jukebox.setChanged()
                        }
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

    private fun processEjectCheck(check: PendingEjectCheck) {
        val world = check.world
        val pos = check.pos
        val key = check.key

        if (world.getBlockState(pos).block != Blocks.JUKEBOX) {
            cleanupAfterEject(key)
            return
        }

        val jukebox = world.getBlockEntity(pos) as? JukeboxBlockEntity
        val stillHasBvc = jukebox != null && !jukebox.theItem.isEmpty && isBvcDisc(jukebox.theItem)
        if (stillHasBvc) return

        cleanupAfterEject(key)

        val cx = pos.x + 0.5
        val cy = pos.y + 0.5
        val cz = pos.z + 0.5
        val box = AABB(cx - 2.0, cy - 2.0, cz - 2.0, cx + 2.0, cy + 2.0, cz + 2.0)
        val items = world.getEntitiesOfClass(ItemEntity::class.java, box) { entity ->
            val stack = entity.item
            stack.item == Items.MUSIC_DISC_5 && isBvcDisc(stack)
        }
        for (entity in items) {
            val stack = entity.item
            restoreJukeboxPlayable(stack)
            entity.item = stack
        }
    }

    private fun cleanupAfterEject(key: String) {
        if (audioPlayerManager.hasActivePlayback(key)) {
            audioPlayerManager.stopPlayback(key)
        }
        activeJukeboxes.remove(key)
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

            val audioId = getAudioId(stack) ?: return
            val dimensionId = getDimensionId(world)
            val worldUuid = listener.worldUuidResolver(world)
            val key = listener.audioPlayerManager.locationKey(worldUuid, pos.x, pos.y, pos.z)

            if (listener.audioPlayerManager.hasActivePlayback(key)) return

            world.setBlock(pos, world.getBlockState(pos).setValue(JukeboxBlock.HAS_RECORD, true), 3)

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

        @JvmStatic
        fun isPlaybackActive(jukebox: JukeboxBlockEntity): Boolean {
            val listener = instance ?: return false
            val world = jukebox.level as? ServerLevel ?: return false
            val pos = jukebox.blockPos
            val worldUuid = listener.worldUuidResolver(world)
            val key = listener.audioPlayerManager.locationKey(worldUuid, pos.x, pos.y, pos.z)
            return listener.audioPlayerManager.hasActivePlayback(key)
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
            CustomData.update(DataComponents.CUSTOM_DATA, disc) { nbt ->
                nbt.putBoolean(BVC_DISC_TAG, true)
                nbt.putString(AUDIO_ID_TAG, audioId)
            }
            disc.remove(DataComponents.JUKEBOX_PLAYABLE)
            return disc
        }

        private fun ejectDisc(world: ServerLevel, pos: BlockPos, jukebox: JukeboxBlockEntity) {
            val disc = jukebox.theItem
            if (disc.isEmpty) return
            restoreJukeboxPlayable(disc)
            jukebox.setSongItemWithoutPlaying(ItemStack.EMPTY)
            jukebox.setChanged()
            world.setBlock(pos, world.getBlockState(pos).setValue(JukeboxBlock.HAS_RECORD, false), 3)
            Containers.dropItemStack(world, pos.x + 0.5, pos.y + 1.0, pos.z + 0.5, disc)
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
