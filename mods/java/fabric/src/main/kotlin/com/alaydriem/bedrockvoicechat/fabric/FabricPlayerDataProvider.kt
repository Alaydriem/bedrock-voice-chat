package com.alaydriem.bedrockvoicechat.fabric

import com.alaydriem.bedrockvoicechat.api.PlayerDataProvider
import com.alaydriem.bedrockvoicechat.dto.Dimension
import com.alaydriem.bedrockvoicechat.dto.GameType
import com.alaydriem.bedrockvoicechat.dto.PlayerData
import com.alaydriem.bedrockvoicechat.integration.FloodgateIntegration
import com.alaydriem.bedrockvoicechat.integration.PlayerIdentity
import com.alaydriem.bedrockvoicechat.integration.PlayerIdentityResolver
import com.alaydriem.bedrockvoicechat.svc.RelayWorld
import net.minecraft.server.MinecraftServer
import net.minecraft.server.level.ServerPlayer
import net.minecraft.server.level.ServerLevel
import java.io.File
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap

/**
 * Fabric-specific player data provider using Minecraft server API.
 * Uses event-driven player tracking via ConcurrentHashMap.
 * Stores UUIDs and looks up fresh player references each tick to avoid stale entity references.
 */
class FabricPlayerDataProvider(
    // The Xbox gamertag Floodgate holds for a Bedrock player, null for anyone it does
    // not know. A lambda rather than the integration itself: that class resolves an
    // optional API by reflection and this needs one answer out of it.
    private val xboxGamertagOf: (UUID) -> String? = FloodgateIntegration()::getXboxGamertag,
    // Stamped on every player so this server can peer at all. A player without it
    // has no world the peer boundary can scope, and their audio is refused there
    // rather than carried into someone else's proximity.
    private val relayWorld: RelayWorld? = null,
    // Answers whether a bridge on this server holds the player's voice connection. The BVC
    // server only counts the connections it terminates itself, so without this a bridged
    // player is reported as being in the world with no voice, and every BVC client near
    // them is told they cannot hear you.
    private val bridgedVoice: (UUID) -> Boolean = { false }
) : PlayerDataProvider {
    private val resolver = PlayerIdentityResolver(xboxGamertagOf)
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
                        identity = identity,
                        x = 0.0,
                        y = 0.0,
                        z = 0.0,
                        yaw = 0f,
                        pitch = 0f,
                        dimension = Dimension.Minecraft.DEATH,
                        whispering = false,
                        spectator = false,
                        worldUuid = worldUuid,
                        playerUuid = playerUuid,
                        relayWorldUuid = relayWorld?.id()
                    )
                } else {
                    // Normal player data
                    val dimension = getDimensionFromPlayer(player)
                    PlayerData(
                        identity = identity,
                        x = player.x,
                        y = player.y,
                        z = player.z,
                        yaw = player.yRot,
                        pitch = player.xRot,
                        dimension = dimension,
                        whispering = player.isShiftKeyDown,
                        spectator = player.isSpectator,
                        worldUuid = worldUuid,
                        playerUuid = playerUuid,
                        relayWorldUuid = relayWorld?.id(),
                        bridgedVoice = bridgedVoice(player.getUUID())
                    )
                }
            }
    }

    fun resolveCanonicalName(player: ServerPlayer): String = resolveIdentity(player).name

    /**
     * The online player a canonical name belongs to, or null when nobody here answers
     * to it.
     *
     * The inverse of [resolveCanonicalName]: a name that went out through that method is
     * the only kind that comes back. A Java name is accepted too, because other mods
     * name a Floodgate player by it. An exact gamertag wins over a Java name, so a linked
     * player whose Java name is someone else's gamertag never takes that player's audio.
     */
    fun findByIdentity(name: String): ServerPlayer? {
        val srv = server ?: return null

        return resolver.bestMatch(name, srv.playerList.players.map { it to resolveIdentity(it) })
    }

    /**
     * The key the BVC server indexes this player under, as `game:gamertag`.
     *
     * Anything asking the server about a player wants this; anything naming a player to
     * another player wants [resolveCanonicalName]. One key: the BVC client always signs in
     * with the Xbox gamertag, so the server holds a Floodgate player under it alone.
     */
    fun resolveMembershipKeys(player: ServerPlayer): List<String> =
        listOf(getGameType().membershipKey(resolveIdentity(player).name))

    private fun resolveIdentity(player: ServerPlayer): PlayerIdentity =
        resolver.resolve(player.getUUID(), player.name.string)

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
