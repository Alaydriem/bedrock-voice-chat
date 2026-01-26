package com.alaydriem.bedrockvoicechat.hytale.systems

import com.hypixel.hytale.server.core.universe.PlayerRef
import java.util.UUID
import java.util.function.BiConsumer
import java.util.logging.Level
import java.util.logging.Logger

/**
 * Kotlin implementation of spectator mode detection.
 * Extends Java base class to avoid generic type erasure issues.
 */
class SpectatorChangeSystem(
    private val onSpectatorChange: BiConsumer<UUID, Boolean>
) : AbstractSpectatorChangeSystem() {

    private val logger: Logger = Logger.getLogger(SpectatorChangeSystem::class.java.name)

    override fun onSpectatorChanged(playerRef: PlayerRef, spectator: Boolean) {
        if (spectator) {
            logger.log(Level.FINE, "[BVC] Player '${playerRef.username}' entered SPECTATOR mode")
        } else {
            logger.log(Level.FINE, "[BVC] Player '${playerRef.username}' exited SPECTATOR mode")
        }
        onSpectatorChange.accept(playerRef.uuid, spectator)
    }
}
