package com.alaydriem.bedrockvoicechat.paper

import com.alaydriem.bedrockvoicechat.api.PlayerDataProvider
import com.alaydriem.bedrockvoicechat.dto.Dimension
import com.alaydriem.bedrockvoicechat.dto.GameType
import com.alaydriem.bedrockvoicechat.dto.PlayerData
import org.bukkit.World
import org.bukkit.entity.Player
import java.util.concurrent.ConcurrentHashMap

/**
 * Paper-specific player data provider using Bukkit API.
 * Uses event-driven player tracking via ConcurrentHashMap.
 */
class PaperPlayerDataProvider : PlayerDataProvider {

    private val onlinePlayers: MutableSet<Player> = ConcurrentHashMap.newKeySet()

    fun addPlayer(player: Player) {
        onlinePlayers.add(player)
    }

    fun removePlayer(player: Player) {
        onlinePlayers.remove(player)
    }

    override fun collectPlayers(): List<PlayerData> {
        return onlinePlayers
            .filter { it.isOnline }
            .map { player ->
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
