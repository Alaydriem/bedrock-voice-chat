package com.alaydriem.bedrockvoicechat.hytale

import com.alaydriem.bedrockvoicechat.api.PlayerDataProvider
import com.alaydriem.bedrockvoicechat.dto.Dimension
import com.alaydriem.bedrockvoicechat.dto.GameType
import com.alaydriem.bedrockvoicechat.dto.PlayerData
import com.hypixel.hytale.server.core.universe.PlayerRef
import java.util.concurrent.ConcurrentHashMap

/**
 * Hytale-specific player data provider using PlayerRef API.
 */
class HytalePlayerDataProvider : PlayerDataProvider {
    private val onlinePlayers: MutableSet<PlayerRef> = ConcurrentHashMap.newKeySet()

    fun addPlayer(player: PlayerRef) {
        onlinePlayers.add(player)
    }

    fun removePlayer(player: PlayerRef) {
        onlinePlayers.remove(player)
    }

    override fun collectPlayers(): List<PlayerData> {
        return onlinePlayers
            .filter { it.isValid }
            .map { toPlayerData(it) }
    }

    override fun getGameType(): GameType = GameType.HYTALE

    private fun toPlayerData(ref: PlayerRef): PlayerData {
        val pos = ref.transform.position
        val rot = ref.headRotation
        val worldUuid = ref.worldUuid.toString()

        return PlayerData(
            name = ref.username,
            x = pos.x, y = pos.y, z = pos.z,
            yaw = rot.x, pitch = rot.y,
            dimension = Dimension.Hytale.ORBIS,
            worldUuid = worldUuid
        )
    }
}
