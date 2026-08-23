package com.alaydriem.bedrockvoicechat.fabric.svc

import com.alaydriem.bedrockvoicechat.svc.SvcChannelFactory
import de.maxhenkel.voicechat.api.VoicechatServerApi
import de.maxhenkel.voicechat.api.audiochannel.AudioChannel
import net.minecraft.server.MinecraftServer
import net.minecraft.server.level.ServerLevel
import net.minecraft.server.level.ServerPlayer
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
    private val server: MinecraftServer,
    // The inverse of the naming the frame's speaker came from. Matching the raw
    // profile name here instead is not the same lookup, and it fails silently:
    // the speaker falls back to a fixed position, which sounds almost right and
    // animates nobody.
    private val bodyOf: (String) -> ServerPlayer?
) : SvcChannelFactory {

    override fun entityChannel(speaker: String): AudioChannel? {
        val player = bodyOf(speaker) ?: return null

        return api.createEntityAudioChannel(player.uuid, api.fromEntity(player))
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
