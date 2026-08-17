package com.alaydriem.bedrockvoicechat.paper.svc

import com.alaydriem.bedrockvoicechat.svc.SvcChannelFactory
import de.maxhenkel.voicechat.api.VoicechatServerApi
import de.maxhenkel.voicechat.api.audiochannel.AudioChannel
import org.bukkit.Server
import org.bukkit.World
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
    private val server: Server
) : SvcChannelFactory {

    override fun entityChannel(id: UUID, speaker: String): AudioChannel? {
        // Exact match: a prefix match would attach a remote speaker's audio to
        // whichever local player happened to share the start of their name.
        val player = server.getPlayerExact(speaker) ?: return null

        return api.createEntityAudioChannel(id, api.fromEntity(player))
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
            else -> return null
        }

        return server.worlds.firstOrNull { it.environment == environment }
    }
}
