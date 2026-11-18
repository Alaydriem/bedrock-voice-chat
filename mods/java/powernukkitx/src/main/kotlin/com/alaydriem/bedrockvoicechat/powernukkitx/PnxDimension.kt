package com.alaydriem.bedrockvoicechat.powernukkitx

import com.alaydriem.bedrockvoicechat.dto.Dimension
import org.powernukkitx.level.Level

/**
 * Maps a PowerNukkitX level to the dimension BVC reports for it.
 *
 * Several levels can share a dimension id; the world UUID, not the dimension, is
 * what keeps their players apart.
 */
object PnxDimension {

    fun of(level: Level?): Dimension {
        if (level == null) {
            return Dimension.Minecraft.OVERWORLD
        }

        return when (level.getDimension()) {
            Level.DIMENSION_OVERWORLD -> Dimension.Minecraft.OVERWORLD
            Level.DIMENSION_NETHER -> Dimension.Minecraft.NETHER
            Level.DIMENSION_THE_END -> Dimension.Minecraft.THE_END
            else -> Dimension.Custom(level.getFolderName())
        }
    }
}
