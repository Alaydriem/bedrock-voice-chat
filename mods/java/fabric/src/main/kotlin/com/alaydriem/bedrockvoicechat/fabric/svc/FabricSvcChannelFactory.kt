package com.alaydriem.bedrockvoicechat.fabric.svc

import com.alaydriem.bedrockvoicechat.svc.SvcChannelFactory
import de.maxhenkel.voicechat.api.VoicechatServerApi
import de.maxhenkel.voicechat.api.audiochannel.AudioChannel
import net.minecraft.server.MinecraftServer
import net.minecraft.server.level.ServerLevel
import java.util.UUID

/**
 * Opens SVC channels against Minecraft's levels and players.
 *
 * SVC wraps platform objects itself through `fromEntity` and `fromServerLevel`, so
 * this is a lookup rather than an adapter: it decides *which* level or body, and SVC
 * decides how to represent it.
 */
class FabricSvcChannelFactory(
    private val api: VoicechatServerApi,
    private val server: MinecraftServer
) : SvcChannelFactory {

    override fun entityChannel(id: UUID, speaker: String): AudioChannel? {
        // Matched on the canonical name the position feed uses, so a Bedrock player
        // on a Geyser server resolves by gamertag rather than by their prefixed
        // Java username.
        val player = server.playerList.players.firstOrNull { it.gameProfile.name == speaker }
            ?: return null

        return api.createEntityAudioChannel(id, api.fromEntity(player))
    }

    override fun locationalChannel(
        id: UUID,
        dimension: String,
        x: Double,
        y: Double,
        z: Double
    ): AudioChannel? {
        val level = levelFor(dimension) ?: return null

        return api.createLocationalAudioChannel(
            id,
            api.fromServerLevel(level),
            api.createPosition(x, y, z)
        )
    }

    private fun levelFor(dimension: String): ServerLevel? {
        val key = when (dimension) {
            "overworld" -> "minecraft:overworld"
            "nether" -> "minecraft:the_nether"
            "the_end" -> "minecraft:the_end"
            else -> return null
        }

        return server.allLevels.firstOrNull { it.dimension().identifier().toString() == key }
    }
}
