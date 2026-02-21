package com.alaydriem.bedrockvoicechat.hytale

import com.alaydriem.bedrockvoicechat.api.PlayerDataProvider
import com.alaydriem.bedrockvoicechat.dto.Dimension
import com.alaydriem.bedrockvoicechat.dto.GameType
import com.alaydriem.bedrockvoicechat.dto.PlayerData
import com.alaydriem.bedrockvoicechat.hytale.systems.CachedPosition
import com.hypixel.hytale.server.core.universe.PlayerRef
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap

/**
 * Hytale-specific player data provider using ECS system-based state caching.
 *
 * All state (position, crouch, death, spectator) is updated from ECS systems
 * running on the world thread, then read from the async BVC tick thread via
 * thread-safe ConcurrentHashMap caches.
 */
class HytalePlayerDataProvider : PlayerDataProvider {
    private val onlinePlayers: MutableSet<PlayerRef> = ConcurrentHashMap.newKeySet()

    // ECS system-driven state caches (updated from world thread, read from async)
    private val deadPlayers: MutableSet<UUID> = ConcurrentHashMap.newKeySet()
    private val crouchingPlayers: MutableSet<UUID> = ConcurrentHashMap.newKeySet()
    private val spectatorPlayers: MutableSet<UUID> = ConcurrentHashMap.newKeySet()
    private val positionCache: MutableMap<UUID, CachedPosition> = ConcurrentHashMap()

    fun addPlayer(player: PlayerRef) {
        onlinePlayers.add(player)
    }

    fun removePlayer(player: PlayerRef) {
        onlinePlayers.remove(player)
        val uuid = player.uuid
        deadPlayers.remove(uuid)
        crouchingPlayers.remove(uuid)
        spectatorPlayers.remove(uuid)
        positionCache.remove(uuid)
    }

    // Called from DeathChangeSystem on world thread
    fun markDead(playerUuid: UUID) {
        deadPlayers.add(playerUuid)
    }

    fun markAlive(playerUuid: UUID) {
        deadPlayers.remove(playerUuid)
    }

    // Called from CrouchTickingSystem on world thread
    fun setCrouching(playerUuid: UUID, crouching: Boolean) {
        if (crouching) crouchingPlayers.add(playerUuid)
        else crouchingPlayers.remove(playerUuid)
    }

    // Called from SpectatorChangeSystem on world thread
    fun setSpectator(playerUuid: UUID, spectator: Boolean) {
        if (spectator) spectatorPlayers.add(playerUuid)
        else spectatorPlayers.remove(playerUuid)
    }

    // Called from PositionTickingSystem on world thread
    fun updatePosition(playerUuid: UUID, position: CachedPosition) {
        positionCache[playerUuid] = position
    }

    override fun collectPlayers(): List<PlayerData> {
        return onlinePlayers
            .filter { it.isValid }
            .mapNotNull { toPlayerData(it) }
    }

    override fun getGameType(): GameType = GameType.HYTALE

    private fun toPlayerData(ref: PlayerRef): PlayerData? {
        val playerUuid = ref.uuid
        val playerName = ref.username

        if (deadPlayers.contains(playerUuid)) {
            val worldUuid = positionCache[playerUuid]?.worldUuid ?: ""
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

        // Read from world-thread cache only — no direct ECS component access
        val cached = positionCache[playerUuid] ?: return null

        return PlayerData(
            name = playerName,
            x = cached.x, y = cached.y, z = cached.z,
            yaw = cached.yaw, pitch = cached.pitch,
            dimension = Dimension.Hytale.ORBIS,
            worldUuid = cached.worldUuid,
            deafen = crouchingPlayers.contains(playerUuid),
            spectator = spectatorPlayers.contains(playerUuid)
        )
    }
}
