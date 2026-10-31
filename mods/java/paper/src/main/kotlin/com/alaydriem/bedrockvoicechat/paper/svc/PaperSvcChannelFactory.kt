package com.alaydriem.bedrockvoicechat.paper.svc

import com.alaydriem.bedrockvoicechat.svc.SvcChannelFactory
import com.alaydriem.bedrockvoicechat.svc.SvcDimensionResolver
import de.maxhenkel.voicechat.api.VoicechatServerApi
import de.maxhenkel.voicechat.api.audiochannel.AudioChannel
import org.bukkit.Server
import org.bukkit.World
import org.bukkit.entity.Player
import java.util.UUID

/**
 * Opens SVC channels against Bukkit's worlds and players.
 *
 * SVC wraps platform objects itself through `fromEntity` and `fromServerLevel`, so
 * this is a lookup rather than an adapter: it decides *which* world or body, and SVC
 * decides how to represent it.
 */
class PaperSvcChannelFactory(
    private val api: VoicechatServerApi,
    private val server: Server,
    // The inverse of the naming the frame's speaker came from. Matching the raw
    // profile name here instead is not the same lookup, and it fails silently:
    // the speaker falls back to a fixed position, which sounds almost right and
    // animates nobody.
    private val bodyOf: (String) -> Player?,
    private val dimensions: SvcDimensionResolver
) : SvcChannelFactory {

    override fun entityChannel(speaker: String): AudioChannel? {
        val player = bodyOf(speaker) ?: return null

        return api.createEntityAudioChannel(player.uniqueId, api.fromEntity(player))
    }

    override fun locationalChannel(
        id: UUID,
        dimension: String,
        x: Double,
        y: Double,
        z: Double
    ): AudioChannel? {
        val world = worldFor(dimension) ?: return null

        return api.createLocationalAudioChannel(
            id,
            api.fromServerLevel(world),
            api.createPosition(x, y, z)
        )
    }

    /**
     * The first world of the matching environment.
     *
     * A server with several worlds of one environment has no way to tell which the
     * speaker meant: the frame carries a dimension, not a world. Picking the first
     * is a deliberate choice for the multiverse case rather than a claim to be
     * right — and it only affects speakers who are not on this server, since a
     * speaker who is here gets an entity channel instead.
     */
    private fun worldFor(dimension: String): World? {
        val environment = when (dimension) {
            "overworld" -> World.Environment.NORMAL
            "nether" -> World.Environment.NETHER
            "the_end" -> World.Environment.THE_END
            else -> return customWorldFor(dimension)
        }

        return server.worlds.firstOrNull { it.environment == environment }
    }

    // A custom world is named on the wire by the hash of `world.name`, the same name the
    // position feed sends for it.
    private fun customWorldFor(dimension: String): World? {
        if (!dimension.startsWith(CUSTOM_PREFIX)) {
            return null
        }

        val names = server.worlds
            .filter { it.environment == World.Environment.CUSTOM }
            .map { it.name }

        return dimensions.resolve(dimension, names)?.let { server.getWorld(it) }
    }

    private companion object {
        const val CUSTOM_PREFIX = "custom:"
    }
}
