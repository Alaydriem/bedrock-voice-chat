package com.alaydriem.bedrockvoicechat.paper

import com.alaydriem.bedrockvoicechat.api.PlayerDataProvider
import com.alaydriem.bedrockvoicechat.dto.Dimension
import com.alaydriem.bedrockvoicechat.dto.GameType
import com.alaydriem.bedrockvoicechat.dto.PlayerData
import com.alaydriem.bedrockvoicechat.integration.FloodgateIntegration
import com.alaydriem.bedrockvoicechat.integration.PlayerIdentity
import com.alaydriem.bedrockvoicechat.integration.PlayerIdentityResolver
import com.alaydriem.bedrockvoicechat.svc.RelayWorld
import org.bukkit.World
import org.bukkit.entity.Player
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap

/**
 * Paper-specific player data provider using Bukkit API.
 * Uses event-driven player tracking via ConcurrentHashMap.
 * Stores UUIDs and looks up fresh player references each tick to avoid stale entity references.
 */
class PaperPlayerDataProvider(
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
                val identity = resolveIdentity(player)
                val playerUuid = player.uniqueId.toString()

                // Check if player is dead - override to death dimension at origin
                if (deadPlayers.contains(player.uniqueId)) {
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
                        worldUuid = player.location.world?.uid?.toString(),
                        playerUuid = playerUuid,
                        relayWorldUuid = relayWorld?.id()
                    )
                } else {
                    // Normal player data
                    val location = player.location
                    val dimension = getDimension(location.world)
                    PlayerData(
                        identity = identity,
                        x = location.x,
                        y = location.y,
                        z = location.z,
                        yaw = location.yaw,
                        pitch = location.pitch,
                        dimension = dimension,
                        whispering = player.isSneaking,
                        spectator = player.gameMode == org.bukkit.GameMode.SPECTATOR,
                        worldUuid = location.world?.uid?.toString(),
                        playerUuid = playerUuid,
                        relayWorldUuid = relayWorld?.id(),
                        bridgedVoice = bridgedVoice(player.uniqueId)
                    )
                }
            }
    }

    fun resolveCanonicalName(player: Player): String = resolveIdentity(player).name

    /**
     * The online player a canonical name belongs to, or null when nobody here answers
     * to it.
     *
     * The inverse of [resolveCanonicalName]: a name that went out through that method is
     * the only kind that comes back. A Java name is accepted too, because other plugins
     * name a Floodgate player by it. An exact gamertag wins over a Java name, so a linked
     * player whose Java name is someone else's gamertag never takes that player's audio.
     */
    fun findByIdentity(name: String): Player? {
        val srv = server ?: return null

        return resolver.bestMatch(name, srv.onlinePlayers.map { it to resolveIdentity(it) })
    }

    /**
     * The key the BVC server indexes this player under, as `game:gamertag`.
     *
     * Anything asking the server about a player wants this; anything naming a player to
     * another player wants [resolveCanonicalName]. One key: the BVC client always signs in
     * with the Xbox gamertag, so the server holds a Floodgate player under it alone.
     */
    fun resolveMembershipKeys(player: Player): List<String> =
        listOf(getGameType().membershipKey(resolveIdentity(player).name))

    private fun resolveIdentity(player: Player): PlayerIdentity =
        resolver.resolve(player.uniqueId, player.name)

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
