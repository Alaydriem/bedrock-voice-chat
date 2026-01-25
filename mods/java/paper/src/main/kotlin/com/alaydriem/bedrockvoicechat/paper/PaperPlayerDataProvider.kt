package com.alaydriem.bedrockvoicechat.paper

import com.alaydriem.bedrockvoicechat.api.PlayerDataProvider
import com.alaydriem.bedrockvoicechat.dto.Dimension
import com.alaydriem.bedrockvoicechat.dto.GameType
import com.alaydriem.bedrockvoicechat.dto.PlayerData
import org.bukkit.World
import org.bukkit.entity.Player
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap

/**
 * Paper-specific player data provider using Bukkit API.
 * Uses event-driven player tracking via ConcurrentHashMap.
 * Stores UUIDs and looks up fresh player references each tick to avoid stale entity references.
 */
class PaperPlayerDataProvider : PlayerDataProvider {
    var server: org.bukkit.Server? = null

    private val onlinePlayers: MutableSet<UUID> = ConcurrentHashMap.newKeySet()
    private val deadPlayers: MutableSet<UUID> = ConcurrentHashMap.newKeySet()

    fun addPlayer(player: Player) {
        onlinePlayers.add(player.uniqueId)
    }

    fun removePlayer(player: Player) {
        onlinePlayers.remove(player.uniqueId)
        deadPlayers.remove(player.uniqueId)
    }

    fun markDead(player: Player) {
        deadPlayers.add(player.uniqueId)
    }

    fun markAlive(player: Player) {
        deadPlayers.remove(player.uniqueId)
    }

    override fun collectPlayers(): List<PlayerData> {
        val srv = server ?: return emptyList()

        return onlinePlayers
            .mapNotNull { uuid -> srv.getPlayer(uuid) }
            .filter { it.isOnline }
            .map { player ->
                // Check if player is dead - override to death dimension at origin
                if (deadPlayers.contains(player.uniqueId)) {
                    PlayerData(
                        name = player.name,
                        x = 0.0,
                        y = 0.0,
                        z = 0.0,
                        yaw = 0f,
                        pitch = 0f,
                        dimension = Dimension.Minecraft.DEATH,
                        deafen = false
                    )
                } else {
                    // Normal player data
                    val location = player.location
                    val dimension = getDimension(location.world)
                    PlayerData(
                        name = player.name,
                        x = location.x,
                        y = location.y,
                        z = location.z,
                        yaw = location.yaw,
                        pitch = location.pitch,
                        dimension = dimension,
                        deafen = player.isSneaking
                    )
                }
            }
    }

    override fun getGameType(): GameType = GameType.MINECRAFT

    private fun getDimension(world: World?): Dimension {
        if (world == null) {
            return Dimension.Minecraft.OVERWORLD
        }

        return when (world.environment) {
            World.Environment.NORMAL -> Dimension.Minecraft.OVERWORLD
            World.Environment.NETHER -> Dimension.Minecraft.NETHER
            World.Environment.THE_END -> Dimension.Minecraft.THE_END
            World.Environment.CUSTOM -> Dimension.Custom(world.name)
        }
    }
}
