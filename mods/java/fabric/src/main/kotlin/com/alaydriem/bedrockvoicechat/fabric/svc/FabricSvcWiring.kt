package com.alaydriem.bedrockvoicechat.fabric.svc

import com.alaydriem.bedrockvoicechat.fabric.FabricPlayerDataProvider
import com.alaydriem.bedrockvoicechat.svc.SpeakerSnapshot
import net.minecraft.server.MinecraftServer
import java.util.UUID

/**
 * The platform-shaped halves of the bridge on Fabric.
 *
 * Kept beside the channel factory rather than in the mod class, so the entry point
 * wires the bridge in one line and this file holds everything Minecraft-specific
 * about it.
 */
class FabricSvcWiring(
    private val server: MinecraftServer,
    private val players: FabricPlayerDataProvider
) {
    /**
     * Where a speaker is, read at the moment a frame is built.
     *
     * The name comes from the same identity resolution the position feed uses, so a
     * Bedrock player on a Geyser server is named by their gamertag on both paths
     * rather than by their prefixed Java username on one of them.
     */
    fun speaker(id: UUID): SpeakerSnapshot? {
        val player = server.playerList.getPlayer(id) ?: return null

        return SpeakerSnapshot(
            name = players.resolveCanonicalName(player),
            x = player.x.toFloat(),
            y = player.y.toFloat(),
            z = player.z.toFloat(),
            dimension = dimensionOf(player.level().dimension().identifier().toString())
        )
    }

    private fun dimensionOf(key: String): String = when (key) {
        "minecraft:the_nether" -> "nether"
        "minecraft:the_end" -> "the_end"
        else -> "overworld"
    }
}
