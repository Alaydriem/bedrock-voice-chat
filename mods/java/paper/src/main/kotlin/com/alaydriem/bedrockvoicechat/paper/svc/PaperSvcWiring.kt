package com.alaydriem.bedrockvoicechat.paper.svc

import com.alaydriem.bedrockvoicechat.paper.PaperPlayerDataProvider
import com.alaydriem.bedrockvoicechat.svc.SpeakerSnapshot
import org.bukkit.Server
import org.bukkit.World
import java.util.UUID

/**
 * The platform-shaped halves of the bridge on Paper.
 *
 * Kept beside the channel factory rather than in the plugin class, so the entry
 * point wires the bridge in one line and this file holds everything Bukkit-specific
 * about it.
 */
class PaperSvcWiring(
    private val server: Server,
    private val players: PaperPlayerDataProvider
) {
    /**
     * Where a speaker is, read at the moment a frame is built.
     *
     * The name comes from the same identity resolution the position feed uses, so a
     * Bedrock player on a Geyser server is named by their gamertag on both paths
     * rather than by their prefixed Java username on one of them.
     */
    fun speaker(id: UUID): SpeakerSnapshot? {
        val player = server.getPlayer(id) ?: return null
        if (!player.isOnline) {
            return null
        }

        val location = player.location

        return SpeakerSnapshot(
            name = players.resolveCanonicalName(player),
            x = location.x.toFloat(),
            y = location.y.toFloat(),
            z = location.z.toFloat(),
            dimension = dimensionOf(location.world)
        )
    }

    private fun dimensionOf(world: World?): String = when (world?.environment) {
        World.Environment.NETHER -> "nether"
        World.Environment.THE_END -> "the_end"
        else -> "overworld"
    }
}
