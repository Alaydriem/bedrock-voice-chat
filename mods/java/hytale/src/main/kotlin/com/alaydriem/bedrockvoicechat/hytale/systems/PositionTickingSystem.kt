package com.alaydriem.bedrockvoicechat.hytale.systems

import com.hypixel.hytale.component.ArchetypeChunk
import com.hypixel.hytale.component.CommandBuffer
import com.hypixel.hytale.component.Store
import com.hypixel.hytale.component.query.Query
import com.hypixel.hytale.component.system.tick.EntityTickingSystem
import com.hypixel.hytale.server.core.universe.PlayerRef
import com.hypixel.hytale.server.core.universe.world.storage.EntityStore
import java.util.UUID

data class CachedPosition(
    val x: Double, val y: Double, val z: Double,
    val yaw: Float, val pitch: Float,
    val worldUuid: String
)

/**
 * Tick-based system that caches player position and rotation on the world thread.
 * The async BVC tick reads from this cache instead of accessing ECS components directly.
 */
class PositionTickingSystem(
    private val onPositionUpdate: (UUID, CachedPosition) -> Unit
) : EntityTickingSystem<EntityStore>() {

    private val query: Query<EntityStore> = Query.and(
        PlayerRef.getComponentType()
    )

    override fun getQuery(): Query<EntityStore> = query

    override fun tick(
        deltaTime: Float,
        index: Int,
        chunk: ArchetypeChunk<EntityStore>,
        store: Store<EntityStore>,
        commandBuffer: CommandBuffer<EntityStore>
    ) {
        try {
            val playerRef = chunk.getComponent(index, PlayerRef.getComponentType()) ?: return
            playerRef.reference ?: return
            val transform = playerRef.transform ?: return
            val pos = transform.position ?: return
            val rot = playerRef.headRotation

            // Hytale headRotation: x=pitch, y=yaw in radians
            // BVC Orientation: x=pitch (up/down, ±90°), y=yaw (facing direction, ±180°)
            val pitchDeg = Math.toDegrees((rot?.x ?: 0f).toDouble()).toFloat()
            val yawDeg = Math.toDegrees((rot?.y ?: 0f).toDouble()).toFloat()

            onPositionUpdate(
                playerRef.uuid,
                CachedPosition(
                    x = pos.x, y = pos.y, z = pos.z,
                    yaw = yawDeg, pitch = pitchDeg,
                    worldUuid = playerRef.worldUuid?.toString() ?: ""
                )
            )
        } catch (_: Exception) {
            return
        }
    }
}
