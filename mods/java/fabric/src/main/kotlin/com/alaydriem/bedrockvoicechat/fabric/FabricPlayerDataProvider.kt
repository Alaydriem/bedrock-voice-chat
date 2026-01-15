package com.alaydriem.bedrockvoicechat.fabric

import com.alaydriem.bedrockvoicechat.api.PlayerDataProvider
import com.alaydriem.bedrockvoicechat.dto.Dimension
import com.alaydriem.bedrockvoicechat.dto.GameType
import com.alaydriem.bedrockvoicechat.dto.PlayerData
import net.minecraft.server.MinecraftServer
import net.minecraft.server.network.ServerPlayerEntity
import net.minecraft.server.world.ServerWorld
import java.util.concurrent.ConcurrentHashMap

/**
 * Fabric-specific player data provider using Minecraft server API.
 * Uses event-driven player tracking via ConcurrentHashMap.
 */
class FabricPlayerDataProvider : PlayerDataProvider {
    var server: MinecraftServer? = null

    private val onlinePlayers: MutableSet<ServerPlayerEntity> = ConcurrentHashMap.newKeySet()

    fun addPlayer(player: ServerPlayerEntity) {
        onlinePlayers.add(player)
    }

    fun removePlayer(player: ServerPlayerEntity) {
        onlinePlayers.remove(player)
    }

    override fun collectPlayers(): List<PlayerData> {
        return onlinePlayers
            .filter { !it.isDisconnected }
            .map { player ->
            val dimension = getDimensionFromPlayer(player)
            PlayerData(
                name = player.name.string,
                x = player.x,
                y = player.y,
                z = player.z,
                yaw = player.yaw,
                pitch = player.pitch,
                dimension = dimension,
                deafen = player.isSneaking
            )
        }
    }

    override fun getGameType(): GameType = GameType.MINECRAFT

    private fun getDimensionFromPlayer(player: ServerPlayerEntity): Dimension {
        val dimensionId = player.entityWorld.registryKey.value.toString()

        return when (dimensionId) {
            "minecraft:overworld" -> Dimension.Minecraft.OVERWORLD
            "minecraft:the_nether" -> Dimension.Minecraft.NETHER
            "minecraft:the_end" -> Dimension.Minecraft.THE_END
            else -> Dimension.Custom(dimensionId)
        }
    }
}
