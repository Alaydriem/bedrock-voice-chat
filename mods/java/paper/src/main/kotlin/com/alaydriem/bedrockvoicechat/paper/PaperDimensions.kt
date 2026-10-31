package com.alaydriem.bedrockvoicechat.paper

import com.alaydriem.bedrockvoicechat.dto.Dimension
import org.bukkit.World

/**
 * The dimension a Paper world is in, shared by positions, the jukebox and the SVC bridge
 * so the three always name a world the same way.
 *
 * A custom world is named by `world.name`; the BVC server hashes it into a dimension of
 * its own.
 */
object PaperDimensions {
    fun of(world: World?): Dimension {
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
