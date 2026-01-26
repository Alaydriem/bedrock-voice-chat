package com.alaydriem.bedrockvoicechat.hytale.systems

import com.hypixel.hytale.server.core.universe.PlayerRef
import java.util.UUID
import java.util.function.Consumer
import java.util.logging.Level
import java.util.logging.Logger

/**
 * Kotlin implementation of death detection system.
 * Extends Java base class to avoid generic type erasure issues.
 */
class DeathChangeSystem(
    private val onDeath: Consumer<UUID>,
    private val onRespawn: Consumer<UUID>
) : AbstractDeathChangeSystem() {

    private val logger: Logger = Logger.getLogger(DeathChangeSystem::class.java.name)

    override fun onPlayerDied(playerRef: PlayerRef) {
        logger.log(Level.FINE, "[BVC] Player '${playerRef.username}' DIED")
        onDeath.accept(playerRef.uuid)
    }

    override fun onPlayerRespawned(playerRef: PlayerRef) {
        logger.log(Level.FINE, "[BVC] Player '${playerRef.username}' RESPAWNED")
        onRespawn.accept(playerRef.uuid)
    }
}
