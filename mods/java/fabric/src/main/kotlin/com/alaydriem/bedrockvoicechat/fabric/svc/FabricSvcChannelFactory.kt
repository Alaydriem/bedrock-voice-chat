package com.alaydriem.bedrockvoicechat.fabric.svc

import com.alaydriem.bedrockvoicechat.svc.SvcChannelFactory
import de.maxhenkel.voicechat.api.VoicechatServerApi
import de.maxhenkel.voicechat.api.audiochannel.AudioChannel
import net.minecraft.server.MinecraftServer
import net.minecraft.server.level.ServerLevel
import net.minecraft.server.level.ServerPlayer
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap

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
    private val bodyOf: (String) -> ServerPlayer?,
    // The relay codec: a raw dimension name to the form frames carry.
    private val wireForm: (String) -> String
) : SvcChannelFactory {

    // One entry per level identifier. A failed open is retried on every frame, so without it
    // a speaker in a dimension this server lacks costs a codec call per level per frame.
    private val forms = ConcurrentHashMap<String, String>()

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

    // The level whose identifier, as the server's codec writes it, is the frame's dimension.
    // The position feed sends the same identifier for it.
    private fun levelFor(dimension: String): ServerLevel? =
        server.allLevels.firstOrNull { level ->
            val identifier = level.dimension().identifier().toString()
            forms.getOrPut(identifier) { wireForm(identifier) } == dimension
        }
}
