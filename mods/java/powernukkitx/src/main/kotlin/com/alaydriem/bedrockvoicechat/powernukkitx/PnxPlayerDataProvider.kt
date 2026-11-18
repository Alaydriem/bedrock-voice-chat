package com.alaydriem.bedrockvoicechat.powernukkitx

import com.alaydriem.bedrockvoicechat.api.PlayerDataProvider
import com.alaydriem.bedrockvoicechat.dto.Dimension
import com.alaydriem.bedrockvoicechat.dto.GameType
import com.alaydriem.bedrockvoicechat.dto.PlayerData
import com.alaydriem.bedrockvoicechat.svc.RelayWorld
import org.powernukkitx.Player
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap

/**
 * Player positions for PowerNukkitX.
 *
 * Tracks UUIDs from join to quit and looks each player up fresh on every tick, so a
 * stale entity reference is never read. On PowerNukkitX the player's name is their
 * Xbox gamertag, so there is no alias to resolve.
 *
 * Player state is read through accessor methods, not Kotlin property syntax: the
 * PowerNukkitX entity hierarchy also exposes public fields of the same names (`x`,
 * `level`, `yaw`), and Kotlin would bind the property syntax to those fields.
 */
class PnxPlayerDataProvider(
    private val worldIdentity: PnxWorldIdentity,
    // Stamped on every player so this server can declare a world to a BVC peer.
    private val relayWorld: RelayWorld?,
    private val playerLookup: (UUID) -> Player?
) : PlayerDataProvider {

    private val onlinePlayers: MutableSet<UUID> = ConcurrentHashMap.newKeySet()
    private val deadPlayers: MutableSet<UUID> = ConcurrentHashMap.newKeySet()

    fun addPlayer(player: Player) {
        onlinePlayers.add(player.getUniqueId())
    }

    fun removePlayer(player: Player) {
        onlinePlayers.remove(player.getUniqueId())
        deadPlayers.remove(player.getUniqueId())
    }

    fun markDead(player: Player) {
        deadPlayers.add(player.getUniqueId())
    }

    fun markAlive(player: Player) {
        deadPlayers.remove(player.getUniqueId())
    }

    fun resolveCanonicalName(player: Player): String = player.getName()

    fun worldUuidOf(player: Player): String? =
        player.getLevel()?.let { worldIdentity.uuidFor(it.getFolderName()) }

    override fun collectPlayers(): List<PlayerData> =
        onlinePlayers
            .mapNotNull(playerLookup)
            .filter { it.isOnline() }
            .map(::toPlayerData)

    override fun getGameType(): GameType = GameType.MINECRAFT

    private fun toPlayerData(player: Player): PlayerData {
        val playerUuid = player.getUniqueId()
        val worldUuid = worldUuidOf(player)

        // A dead player is moved to the death dimension at the origin, so nobody
        // hears them until they respawn.
        if (deadPlayers.contains(playerUuid)) {
            return PlayerData(
                name = player.getName(),
                x = 0.0,
                y = 0.0,
                z = 0.0,
                yaw = 0f,
                pitch = 0f,
                dimension = Dimension.Minecraft.DEATH,
                whispering = false,
                spectator = false,
                worldUuid = worldUuid,
                playerUuid = playerUuid.toString(),
                relayWorldUuid = relayWorld?.id()
            )
        }

        return PlayerData(
            name = player.getName(),
            x = player.getX(),
            y = player.getY(),
            z = player.getZ(),
            yaw = player.getYaw().toFloat(),
            pitch = player.getPitch().toFloat(),
            dimension = PnxDimension.of(player.getLevel()),
            whispering = player.isSneaking(),
            spectator = player.isSpectator(),
            worldUuid = worldUuid,
            playerUuid = playerUuid.toString(),
            relayWorldUuid = relayWorld?.id()
        )
    }
}
