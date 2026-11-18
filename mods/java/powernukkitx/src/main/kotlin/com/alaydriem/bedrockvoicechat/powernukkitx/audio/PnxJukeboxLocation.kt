package com.alaydriem.bedrockvoicechat.powernukkitx.audio

import org.powernukkitx.level.Level
import org.powernukkitx.math.Vector3

/**
 * A jukebox block position in one level.
 */
data class PnxJukeboxLocation(val level: Level, val x: Int, val y: Int, val z: Int) {

    val chunkX: Int get() = x shr 4

    val chunkZ: Int get() = z shr 4

    fun vector(): Vector3 = Vector3(x.toDouble(), y.toDouble(), z.toDouble())

    fun offset(dx: Int, dy: Int, dz: Int): PnxJukeboxLocation = copy(x = x + dx, y = y + dy, z = z + dz)
}
