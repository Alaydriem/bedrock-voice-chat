package com.alaydriem.bedrockvoicechat.fabric

import com.alaydriem.bedrockvoicechat.api.PlayerDataProvider
import com.alaydriem.bedrockvoicechat.dto.Dimension
import com.alaydriem.bedrockvoicechat.dto.GameType
import com.alaydriem.bedrockvoicechat.dto.PlayerData
import com.alaydriem.bedrockvoicechat.integration.FloodgateIntegration
import net.minecraft.server.MinecraftServer
import net.minecraft.server.level.ServerLevel
import net.minecraft.server.level.ServerPlayer
import java.io.File
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap

class FabricPlayerDataProvider(
    private val floodgate: FloodgateIntegration = FloodgateIntegration(),
    private val floodgatePrefix: String? = null
) : PlayerDataProvider {
    var server: MinecraftServer? = null

    private val onlinePlayers: MutableSet<UUID> = ConcurrentHashMap.newKeySet()
    private val deadPlayers: MutableSet<UUID> = ConcurrentHashMap.newKeySet()
    private val worldUuidCache = ConcurrentHashMap<String, String>()

    fun addPlayer(player: ServerPlayer) {
        onlinePlayers.add(player.uuid)
    }

    fun removePlayer(player: ServerPlayer) {
        onlinePlayers.remove(player.uuid)
        deadPlayers.remove(player.uuid)
    }

    fun markDead(player: ServerPlayer) {
        deadPlayers.add(player.uuid)
    }

    fun markAlive(player: ServerPlayer) {
        deadPlayers.remove(player.uuid)
    }

    override fun collectPlayers(): List<PlayerData> {
        val srv = server ?: return emptyList()

        return onlinePlayers
            .mapNotNull { uuid -> srv.playerList.getPlayer(uuid) }
            .filter { !it.hasDisconnected() }
            .map { player ->
                val worldUuid = getWorldUuid(player.level())
                val altIdentity = resolveAlternativeIdentity(player)
                val playerUuid = player.uuid.toString()

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
                        spectator = false,
                        worldUuid = worldUuid,
                        alternativeIdentity = altIdentity,
                        playerUuid = playerUuid
                    )
                } else {
                    val dimension = getDimensionFromPlayer(player)
                    PlayerData(
                        name = player.name.string,
                        x = player.x,
                        y = player.y,
                        z = player.z,
                        yaw = player.yRot,
                        pitch = player.xRot,
                        dimension = dimension,
                        deafen = player.isShiftKeyDown,
                        spectator = player.isSpectator,
                        worldUuid = worldUuid,
                        alternativeIdentity = altIdentity,
                        playerUuid = playerUuid
                    )
                }
            }
    }

    private fun resolveAlternativeIdentity(player: ServerPlayer): String? {
        val playerName = player.name.string
        val result = resolveRawAlternativeIdentity(player, playerName)
        if (result != null && result == playerName) return null
        return result
    }

    private fun resolveRawAlternativeIdentity(player: ServerPlayer, playerName: String): String? {
        val floodgateGamertag = floodgate.getXboxGamertag(player.uuid)
        if (floodgateGamertag != null) return floodgateGamertag

        val prefix = floodgatePrefix
        if (prefix != null && playerName.startsWith(prefix)) {
            return playerName.removePrefix(prefix)
        }

        return null
    }

    override fun getGameType(): GameType = GameType.MINECRAFT

    fun getWorldUuid(world: ServerLevel): String {
        val dimKey = world.dimension().identifier().toString()
        return worldUuidCache.getOrPut(dimKey) {
            val worldDir = world.server.serverDirectory.resolve("bvc").toFile()
            worldDir.mkdirs()
            val uuidFile = File(worldDir, "world_uuid_${dimKey.replace(":", "_")}.txt")
            if (uuidFile.exists()) {
                uuidFile.readText().trim()
            } else {
                val newUuid = UUID.randomUUID().toString()
                uuidFile.writeText(newUuid)
                newUuid
            }
        }
    }

    private fun getDimensionFromPlayer(player: ServerPlayer): Dimension {
        val dimensionId = (player.level()).dimension().identifier().toString()

        return when (dimensionId) {
            "minecraft:overworld" -> Dimension.Minecraft.OVERWORLD
            "minecraft:the_nether" -> Dimension.Minecraft.NETHER
            "minecraft:the_end" -> Dimension.Minecraft.THE_END
            else -> Dimension.Custom(dimensionId)
        }
    }
}
