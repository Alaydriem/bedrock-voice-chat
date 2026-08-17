package com.alaydriem.bedrockvoicechat.fabric

import com.alaydriem.bedrockvoicechat.api.PlayerDataProvider
import com.alaydriem.bedrockvoicechat.dto.Dimension
import com.alaydriem.bedrockvoicechat.dto.GameType
import com.alaydriem.bedrockvoicechat.dto.PlayerData
import com.alaydriem.bedrockvoicechat.integration.FloodgateIntegration
import com.alaydriem.bedrockvoicechat.svc.RelayWorld
import net.minecraft.server.MinecraftServer
import net.minecraft.server.level.ServerPlayer
import net.minecraft.server.level.ServerLevel
import org.slf4j.LoggerFactory
import java.io.File
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap

/**
 * Fabric-specific player data provider using Minecraft server API.
 * Uses event-driven player tracking via ConcurrentHashMap.
 * Stores UUIDs and looks up fresh player references each tick to avoid stale entity references.
 */
class FabricPlayerDataProvider(
    private val floodgate: FloodgateIntegration = FloodgateIntegration(),
    // Stamped on every player so this server can peer at all. A player without it
    // has no world the peer boundary can scope, and their audio is refused there
    // rather than carried into someone else's proximity.
    private val relayWorld: RelayWorld? = null
) : PlayerDataProvider {
    private val log = LoggerFactory.getLogger("BedrockVoiceChat.Identity")
    private val loggedIdentities: MutableSet<String> = ConcurrentHashMap.newKeySet()
    var server: MinecraftServer? = null

    private val onlinePlayers: MutableSet<UUID> = ConcurrentHashMap.newKeySet()
    private val deadPlayers: MutableSet<UUID> = ConcurrentHashMap.newKeySet()
    private val worldUuidCache = ConcurrentHashMap<String, String>()

    fun addPlayer(player: ServerPlayer) {
        onlinePlayers.add(player.getUUID())
    }

    fun removePlayer(player: ServerPlayer) {
        onlinePlayers.remove(player.getUUID())
        deadPlayers.remove(player.getUUID())
    }

    fun markDead(player: ServerPlayer) {
        deadPlayers.add(player.getUUID())
    }

    fun markAlive(player: ServerPlayer) {
        deadPlayers.remove(player.getUUID())
    }

    override fun collectPlayers(): List<PlayerData> {
        val srv = server ?: return emptyList()

        return onlinePlayers
            .mapNotNull { uuid -> srv.playerList.getPlayer(uuid) }
            .filter { !it.hasDisconnected() }
            .map { player ->
                val worldUuid = getWorldUuid(player.level() as ServerLevel)
                val identity = resolveIdentity(player)
                val playerUuid = player.getUUID().toString()

                // Check if player is dead - override to death dimension at origin
                if (deadPlayers.contains(player.getUUID())) {
                    PlayerData(
                        name = identity.name,
                        x = 0.0,
                        y = 0.0,
                        z = 0.0,
                        yaw = 0f,
                        pitch = 0f,
                        dimension = Dimension.Minecraft.DEATH,
                        deafen = false,
                        spectator = false,
                        worldUuid = worldUuid,
                        alternativeIdentity = identity.alternative,
                        playerUuid = playerUuid,
                        relayWorldUuid = relayWorld?.id()
                    )
                } else {
                    // Normal player data
                    val dimension = getDimensionFromPlayer(player)
                    PlayerData(
                        name = identity.name,
                        x = player.x,
                        y = player.y,
                        z = player.z,
                        yaw = player.yRot,
                        pitch = player.xRot,
                        dimension = dimension,
                        deafen = player.isShiftKeyDown,
                        spectator = player.isSpectator,
                        worldUuid = worldUuid,
                        alternativeIdentity = identity.alternative,
                        playerUuid = playerUuid,
                        relayWorldUuid = relayWorld?.id()
                    )
                }
            }
    }

    fun resolveCanonicalName(player: ServerPlayer): String = resolveIdentity(player).name

    private data class Identity(val name: String, val alternative: String?)

    private fun resolveIdentity(player: ServerPlayer): Identity {
        val javaName = player.name.string
        val canonical = floodgate.getXboxGamertag(player.getUUID())
        val identity = when {
            canonical == null -> Identity(javaName, null)
            javaName == canonical -> Identity(javaName, null)
            // Floodgate prefix is applied to this player — strip it so they register under their canonical Xbox gamertag
            javaName.endsWith(canonical) -> Identity(canonical, null)
            // Linked Bedrock player with a different Java account name — keep the Java identity, expose canonical as alias
            else -> Identity(javaName, canonical)
        }
        val logKey = "${player.getUUID()}|$javaName|$canonical"
        if (loggedIdentities.add(logKey)) {
            log.info(
                "Identity resolution: uuid={}, javaName='{}', canonical='{}' -> name='{}', alt='{}'",
                player.getUUID(), javaName, canonical, identity.name, identity.alternative
            )
        }
        return identity
    }

    override fun getGameType(): GameType = GameType.MINECRAFT

    fun getWorldUuid(world: ServerLevel): String {
        val dimKey = world.dimension().identifier().toString()
        return worldUuidCache.getOrPut(dimKey) {
            val worldDir = world.server!!.getServerDirectory().resolve("bvc").toFile()
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
        val dimensionId = player.level().dimension().identifier().toString()

        return when (dimensionId) {
            "minecraft:overworld" -> Dimension.Minecraft.OVERWORLD
            "minecraft:the_nether" -> Dimension.Minecraft.NETHER
            "minecraft:the_end" -> Dimension.Minecraft.THE_END
            else -> Dimension.Custom(dimensionId)
        }
    }
}
