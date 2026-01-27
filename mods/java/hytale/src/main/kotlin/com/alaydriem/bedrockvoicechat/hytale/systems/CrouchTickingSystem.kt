package com.alaydriem.bedrockvoicechat.hytale.systems

import com.hypixel.hytale.component.ArchetypeChunk
import com.hypixel.hytale.component.CommandBuffer
import com.hypixel.hytale.component.Store
import com.hypixel.hytale.component.query.Query
import com.hypixel.hytale.component.system.tick.EntityTickingSystem
import com.hypixel.hytale.server.core.entity.movement.MovementStatesComponent
import com.hypixel.hytale.server.core.universe.PlayerRef
import com.hypixel.hytale.server.core.universe.world.storage.EntityStore
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap
import java.util.function.BiConsumer
import java.util.logging.Level
import java.util.logging.Logger

/**
 * Tick-based system for detecting crouch state changes.
 * EntityTickingSystem ticks every frame for each entity matching the query,
 * allowing us to detect in-place mutations of MovementStatesComponent.
 */
class CrouchTickingSystem(
    private val onCrouchChange: BiConsumer<UUID, Boolean>
) : EntityTickingSystem<EntityStore>() {

    private val logger: Logger = Logger.getLogger(CrouchTickingSystem::class.java.name)
    private val previousCrouchState: MutableMap<UUID, Boolean> = ConcurrentHashMap()

    // Query for entities that have both PlayerRef and MovementStatesComponent
    private val query: Query<EntityStore> = Query.and(
        PlayerRef.getComponentType(),
        MovementStatesComponent.getComponentType()
    )

    override fun getQuery(): Query<EntityStore> = query

    override fun tick(
        deltaTime: Float,
        index: Int,
        chunk: ArchetypeChunk<EntityStore>,
        store: Store<EntityStore>,
        commandBuffer: CommandBuffer<EntityStore>
    ) {
        // Get the PlayerRef component for this entity
        val playerRef = chunk.getComponent(index, PlayerRef.getComponentType()) ?: return

        // Get the MovementStatesComponent
        val movementStatesComponent = chunk.getComponent(index, MovementStatesComponent.getComponentType()) ?: return
        val states = movementStatesComponent.movementStates ?: return

        val playerUuid = playerRef.uuid
        val currentCrouching = states.crouching

        // Check if state changed
        val previousCrouching = previousCrouchState[playerUuid]
        if (previousCrouching == null) {
            // First time seeing this player, store initial state
            previousCrouchState[playerUuid] = currentCrouching
            if (currentCrouching) {
                // If they joined while crouching, report it
                logger.log(Level.FINE, "[BVC] Player '${playerRef.username}' started CROUCHING (initial)")
                onCrouchChange.accept(playerUuid, true)
            }
        } else if (previousCrouching != currentCrouching) {
            // State changed
            previousCrouchState[playerUuid] = currentCrouching
            if (currentCrouching) {
                logger.log(Level.FINE, "[BVC] Player '${playerRef.username}' started CROUCHING")
            } else {
                logger.log(Level.FINE, "[BVC] Player '${playerRef.username}' stopped CROUCHING")
            }
            onCrouchChange.accept(playerUuid, currentCrouching)
        }
    }

    /**
     * Call this when a player disconnects to clean up tracked state.
     */
    fun removePlayer(playerUuid: UUID) {
        previousCrouchState.remove(playerUuid)
    }
}
