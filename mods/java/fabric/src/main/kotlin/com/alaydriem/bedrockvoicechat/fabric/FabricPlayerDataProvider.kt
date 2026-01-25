package com.alaydriem.bedrockvoicechat.fabric

import com.alaydriem.bedrockvoicechat.api.PlayerDataProvider
import com.alaydriem.bedrockvoicechat.dto.Dimension
import com.alaydriem.bedrockvoicechat.dto.GameType
import com.alaydriem.bedrockvoicechat.dto.PlayerData
import net.minecraft.server.MinecraftServer
import net.minecraft.server.network.ServerPlayerEntity
import net.minecraft.server.world.ServerWorld
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap

/**
 * Fabric-specific player data provider using Minecraft server API.
 * Uses event-driven player tracking via ConcurrentHashMap.
 * Stores UUIDs and looks up fresh player references each tick to avoid stale entity references.
 */
class FabricPlayerDataProvider : PlayerDataProvider {
    var server: MinecraftServer? = null

    private val onlinePlayers: MutableSet<UUID> = ConcurrentHashMap.newKeySet()
    private val deadPlayers: MutableSet<UUID> = ConcurrentHashMap.newKeySet()

    fun addPlayer(player: ServerPlayerEntity) {
        onlinePlayers.add(player.uuid)
    }

    fun removePlayer(player: ServerPlayerEntity) {
        onlinePlayers.remove(player.uuid)
        deadPlayers.remove(player.uuid)
    }

    fun markDead(player: ServerPlayerEntity) {
        deadPlayers.add(player.uuid)
    }

    fun markAlive(player: ServerPlayerEntity) {
        deadPlayers.remove(player.uuid)
    }

    override fun collectPlayers(): List<PlayerData> {
        val srv = server ?: return emptyList()

        return onlinePlayers
            .mapNotNull { uuid -> srv.playerManager.getPlayer(uuid) }
            .filter { !it.isDisconnected }
            .map { player ->
                // Check if player is dead - override to death dimension at origin
                if (deadPlayers.contains(player.uuid)) {
                    PlayerData(
                        name = player.name.string,
                        x = 0.0,
                        y = 0.0,
                        z = 0.0,
                        yaw = 0f,
                        pitch = 0f,
                        dimension = Dimension.Minecraft.DEATH,
                        deafen = false,
                        spectator = false
                    )
                } else {
                    // Normal player data
                    val dimension = getDimensionFromPlayer(player)
                    PlayerData(
                        name = player.name.string,
                        x = player.x,
                        y = player.y,
                        z = player.z,
                        yaw = player.yaw,
                        pitch = player.pitch,
                        dimension = dimension,
                        deafen = player.isSneaking,
                        spectator = player.isSpectator
                    )
                }
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
