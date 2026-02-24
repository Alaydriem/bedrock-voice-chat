package com.alaydriem.bedrockvoicechat.fabric.audio

import com.alaydriem.bedrockvoicechat.audio.AudioPlayerManager
import com.alaydriem.bedrockvoicechat.dto.Coordinates
import com.mojang.serialization.MapCodec
import net.minecraft.block.Block
import net.minecraft.block.BlockState
import net.minecraft.block.BlockWithEntity
import net.minecraft.block.entity.BlockEntity
import net.minecraft.entity.player.PlayerEntity
import net.minecraft.item.ItemStack
import net.minecraft.server.world.ServerWorld
import net.minecraft.state.StateManager
import net.minecraft.state.property.BooleanProperty
import net.minecraft.state.property.Properties
import net.minecraft.util.ActionResult
import net.minecraft.util.hit.BlockHitResult
import net.minecraft.util.math.BlockPos
import net.minecraft.world.World
import net.minecraft.world.block.WireOrientation
import org.slf4j.LoggerFactory

/**
 * Custom audio player block for Fabric.
 * Accepts BVC audio discs and uses redstone to control playback.
 * Redstone on = play, off = stop.
 */
class AudioPlayerBlock(settings: Settings) : BlockWithEntity(settings) {

    companion object {
        private val logger = LoggerFactory.getLogger("BVC Audio Block")
        val HAS_DISC: BooleanProperty = BooleanProperty.of("has_disc")
        val POWERED: BooleanProperty = Properties.POWERED
        val CODEC: MapCodec<AudioPlayerBlock> = createCodec(::AudioPlayerBlock)

        // Set by FabricMod during initialization
        var audioPlayerManager: AudioPlayerManager? = null
        var worldUuidProvider: (() -> String?)? = null
    }

    init {
        defaultState = stateManager.defaultState
            .with(HAS_DISC, false)
            .with(POWERED, false)
    }

    override fun getCodec(): MapCodec<out AudioPlayerBlock> = CODEC

    override fun appendProperties(builder: StateManager.Builder<Block, BlockState>) {
        builder.add(HAS_DISC, POWERED)
    }

    override fun createBlockEntity(pos: BlockPos, state: BlockState): BlockEntity {
        return AudioPlayerBlockEntity(pos, state)
    }

    override fun onUse(
        state: BlockState,
        world: World,
        pos: BlockPos,
        player: PlayerEntity,
        hit: BlockHitResult
    ): ActionResult {
        if (world.isClient) return ActionResult.SUCCESS

        val blockEntity = world.getBlockEntity(pos) as? AudioPlayerBlockEntity ?: return ActionResult.PASS
        val manager = audioPlayerManager ?: return ActionResult.PASS
        val worldUuid = worldUuidProvider?.invoke() ?: return ActionResult.PASS
        val locationKey = manager.locationKey(worldUuid, pos.x, pos.y, pos.z)

        if (blockEntity.hasDisc()) {
            // Eject disc
            val disc = blockEntity.removeDisc()
            if (disc != null) {
                manager.removeDisc(locationKey)
                player.giveItemStack(disc)
                world.setBlockState(pos, state.with(HAS_DISC, false))
                logger.info("Disc ejected from audio player at {}", pos)
            }
        } else {
            // Try to insert disc from player's hand
            val heldItem = player.mainHandStack
            if (heldItem.item is AudioDiscItem) {
                val audioId = heldItem.get(FabricAudioRegistry.BVC_AUDIO_ID)
                if (audioId != null) {
                    val discCopy = heldItem.copyWithCount(1)
                    blockEntity.insertDisc(discCopy)
                    heldItem.decrement(1)
                    world.setBlockState(pos, state.with(HAS_DISC, true))

                    manager.insertDisc(locationKey, audioId)

                    // If already powered, start playback immediately
                    if (state.get(POWERED)) {
                        val dim = getDimension(world as? ServerWorld)
                        val coords = Coordinates(pos.x.toDouble(), pos.y.toDouble(), pos.z.toDouble())
                        manager.updatePowerState(locationKey, true, coords, dim, worldUuid)
                    }

                    logger.info("Disc inserted into audio player at {}: {}", pos, audioId)
                }
            }
        }

        return ActionResult.SUCCESS
    }

    override fun neighborUpdate(
        state: BlockState,
        world: World,
        pos: BlockPos,
        sourceBlock: Block,
        wireOrientation: WireOrientation?,
        notify: Boolean
    ) {
        if (world.isClient) return

        val manager = audioPlayerManager ?: return
        val worldUuid = worldUuidProvider?.invoke() ?: return
        val locationKey = manager.locationKey(worldUuid, pos.x, pos.y, pos.z)

        if (!manager.hasDisc(locationKey)) return

        val powered = world.getReceivedRedstonePower(pos) > 0
        val wasPowered = state.get(POWERED)

        if (powered != wasPowered) {
            world.setBlockState(pos, state.with(POWERED, powered))

            val dim = getDimension(world as? ServerWorld)
            val coords = Coordinates(pos.x.toDouble(), pos.y.toDouble(), pos.z.toDouble())
            manager.updatePowerState(locationKey, powered, coords, dim, worldUuid)
        }
    }

    override fun onBreak(world: World, pos: BlockPos, state: BlockState, player: PlayerEntity): BlockState {
        if (!world.isClient) {
            val manager = audioPlayerManager
            val worldUuid = worldUuidProvider?.invoke()

            if (manager != null && worldUuid != null) {
                val locationKey = manager.locationKey(worldUuid, pos.x, pos.y, pos.z)
                manager.onBlockDestroyed(locationKey)
            }

            // Drop the disc
            val blockEntity = world.getBlockEntity(pos) as? AudioPlayerBlockEntity
            val disc = blockEntity?.removeDisc()
            if (disc != null) {
                Block.dropStack(world, pos, disc)
            }
        }

        return super.onBreak(world, pos, state, player)
    }

    private fun getDimension(world: ServerWorld?): String {
        if (world == null) return "overworld"
        val dimId = world.registryKey.value.toString()
        return when (dimId) {
            "minecraft:overworld" -> "overworld"
            "minecraft:the_nether" -> "nether"
            "minecraft:the_end" -> "the_end"
            else -> dimId
        }
    }
}
