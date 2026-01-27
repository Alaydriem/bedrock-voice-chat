package com.alaydriem.bedrockvoicechat.hytale

import com.alaydriem.bedrockvoicechat.api.PlayerDataProvider
import com.alaydriem.bedrockvoicechat.dto.Dimension
import com.alaydriem.bedrockvoicechat.dto.GameType
import com.alaydriem.bedrockvoicechat.dto.PlayerData
import com.hypixel.hytale.server.core.universe.PlayerRef
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap

/**
 * Hytale-specific player data provider using ECS system-based state caching.
 *
 * All state is updated from RefChangeSystem callbacks (running on world thread),
 * then read from async tick thread via thread-safe ConcurrentHashMap caches.
 */
class HytalePlayerDataProvider : PlayerDataProvider {
    private val onlinePlayers: MutableSet<PlayerRef> = ConcurrentHashMap.newKeySet()

    // ECS system-driven state caches (updated from world thread, read from async)
    private val deadPlayers: MutableSet<UUID> = ConcurrentHashMap.newKeySet()
    private val crouchingPlayers: MutableSet<UUID> = ConcurrentHashMap.newKeySet()
    private val spectatorPlayers: MutableSet<UUID> = ConcurrentHashMap.newKeySet()

    fun addPlayer(player: PlayerRef) {
        onlinePlayers.add(player)
    }

    fun removePlayer(player: PlayerRef) {
        onlinePlayers.remove(player)
        deadPlayers.remove(player.uuid)
        crouchingPlayers.remove(player.uuid)
        spectatorPlayers.remove(player.uuid)
    }

    // Called from DeathChangeSystem on world thread
    fun markDead(playerUuid: UUID) {
        deadPlayers.add(playerUuid)
    }

    fun markAlive(playerUuid: UUID) {
        deadPlayers.remove(playerUuid)
    }

    // Called from MovementStatesChangeSystem on world thread
    fun setCrouching(playerUuid: UUID, crouching: Boolean) {
        if (crouching) crouchingPlayers.add(playerUuid)
        else crouchingPlayers.remove(playerUuid)
    }

    // Called from SpectatorChangeSystem on world thread
    fun setSpectator(playerUuid: UUID, spectator: Boolean) {
        if (spectator) spectatorPlayers.add(playerUuid)
        else spectatorPlayers.remove(playerUuid)
    }

    override fun collectPlayers(): List<PlayerData> {
        return onlinePlayers
            .filter { it.isValid }
            .map { toPlayerData(it) }
    }

    override fun getGameType(): GameType = GameType.HYTALE

    // NO getComponent calls - only cache reads and safe PlayerRef properties
    private fun toPlayerData(ref: PlayerRef): PlayerData {
        val playerUuid = ref.uuid
        val playerName = ref.username
        val worldUuid = ref.worldUuid.toString()

        // Check death state from cache
        if (deadPlayers.contains(playerUuid)) {
            return PlayerData(
                name = playerName,
                x = 0.0, y = 0.0, z = 0.0,
                yaw = 0f, pitch = 0f,
                dimension = Dimension.Hytale.DEATH,
                worldUuid = worldUuid,
                deafen = false,
                spectator = false
            )
        }

        val pos = ref.transform.position
        val rot = ref.headRotation

        return PlayerData(
            name = playerName,
            x = pos.x, y = pos.y, z = pos.z,
            yaw = rot.x, pitch = rot.y,
            dimension = Dimension.Hytale.ORBIS,
            worldUuid = worldUuid,
            deafen = crouchingPlayers.contains(playerUuid),
            spectator = spectatorPlayers.contains(playerUuid)
        )
    }
}
