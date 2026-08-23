package com.alaydriem.bedrockvoicechat.paper

import com.alaydriem.bedrockvoicechat.api.PlayerDataProvider
import com.alaydriem.bedrockvoicechat.dto.Dimension
import com.alaydriem.bedrockvoicechat.dto.GameType
import com.alaydriem.bedrockvoicechat.dto.PlayerData
import com.alaydriem.bedrockvoicechat.integration.FloodgateIntegration
import com.alaydriem.bedrockvoicechat.svc.RelayWorld
import org.bukkit.World
import org.bukkit.entity.Player
import org.slf4j.LoggerFactory
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
    private val log = LoggerFactory.getLogger("BedrockVoiceChat.Identity")
    private val loggedIdentities: MutableSet<String> = ConcurrentHashMap.newKeySet()
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
                        name = identity.name,
                        x = 0.0,
                        y = 0.0,
                        z = 0.0,
                        yaw = 0f,
                        pitch = 0f,
                        dimension = Dimension.Minecraft.DEATH,
                        deafen = false,
                        spectator = false,
                        worldUuid = player.location.world?.uid?.toString(),
                        alternativeIdentity = identity.alternative,
                        playerUuid = playerUuid,
                        relayWorldUuid = relayWorld?.id()
                    )
                } else {
                    // Normal player data
                    val location = player.location
                    val dimension = getDimension(location.world)
                    PlayerData(
                        name = identity.name,
                        x = location.x,
                        y = location.y,
                        z = location.z,
                        yaw = location.yaw,
                        pitch = location.pitch,
                        dimension = dimension,
                        deafen = player.isSneaking,
                        spectator = player.gameMode == org.bukkit.GameMode.SPECTATOR,
                        worldUuid = location.world?.uid?.toString(),
                        alternativeIdentity = identity.alternative,
                        playerUuid = playerUuid,
                        relayWorldUuid = relayWorld?.id(),
                        bridgedVoice = bridgedVoice(player.uniqueId)
                    )
                }
            }
    }

    fun resolveCanonicalName(player: Player): String = resolveIdentity(player).name

    /**
     * Every form the BVC server might index this player under, as `game:gamertag`.
     *
     * Anything asking the server about a player wants these; anything naming a player
     * to another player wants [resolveCanonicalName]. Passing a bare name to a lookup
     * answers no rather than failing, which is how a wrong answer survives.
     *
     * More than one, because a linked Bedrock player carries two names: the Java
     * account they log in with and the Xbox gamertag they are known by. The mod sends
     * the first as their name and the second as their alias, while their BVC client
     * authenticates against Xbox Live and so registers under the second. Asking for
     * only one of them misses that player whichever one is chosen.
     */
    /**
     * The online player a canonical name belongs to, or null when nobody here answers
     * to it.
     *
     * The inverse of [resolveCanonicalName], and it has to be: a name that went out
     * through that method is the only kind that comes back. Matching the raw profile
     * name instead misses every Floodgate player, whose profile carries a prefix the
     * canonical form does not — and the miss is silent, because a speaker with no body
     * here is a normal thing that falls back to a fixed position.
     *
     * The alias is accepted too, for a linked player known to the far side by their
     * Xbox gamertag while this server calls them by their Java account.
     */
    fun findByIdentity(name: String): Player? {
        val srv = server ?: return null

        return srv.onlinePlayers.firstOrNull { player ->
            val identity = resolveIdentity(player)
            identity.name.equals(name, ignoreCase = true) ||
                identity.alternative?.equals(name, ignoreCase = true) == true
        }
    }

    fun resolveMembershipKeys(player: Player): List<String> {
        val identity = resolveIdentity(player)
        val game = getGameType()

        return listOfNotNull(identity.name, identity.alternative)
            .distinct()
            .map(game::membershipKey)
    }

    private data class Identity(val name: String, val alternative: String?)

    private fun resolveIdentity(player: Player): Identity {
        val javaName = player.name
        val canonical = xboxGamertagOf(player.uniqueId)
        val identity = when {
            canonical == null -> Identity(javaName, null)
            javaName == canonical -> Identity(javaName, null)
            // Floodgate prefix is applied to this player — strip it so they register under their canonical Xbox gamertag
            javaName.endsWith(canonical) -> Identity(canonical, null)
            // Linked Bedrock player with a different Java account name — keep the Java identity, expose canonical as alias
            else -> Identity(javaName, canonical)
        }
        val logKey = "${player.uniqueId}|$javaName|$canonical"
        if (loggedIdentities.add(logKey)) {
            log.info(
                "Identity resolution: uuid={}, javaName='{}', canonical='{}' -> name='{}', alt='{}'",
                player.uniqueId, javaName, canonical, identity.name, identity.alternative
            )
        }
        return identity
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
