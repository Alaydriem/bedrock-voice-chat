package com.alaydriem.bedrockvoicechat.dto

import com.google.gson.annotations.SerializedName

/**
 * Platform-independent player data DTO for the voice chat API.
 */
data class PlayerData(
    val name: String,
    val coordinates: Coordinates,
    val orientation: Orientation,
    val dimension: String?,
    @SerializedName("world_uuid")
    val worldUuid: String?,
    val deafen: Boolean,
    val spectator: Boolean = false,
    @SerializedName("alternative_identity")
    val alternativeIdentity: String? = null
) {
    /**
     * Constructor for Minecraft players (Fabric/Paper).
     * Accepts an optional worldUuid for multi-world isolation.
     */
    constructor(
        name: String,
        x: Double, y: Double, z: Double,
        yaw: Float, pitch: Float,
        dimension: Dimension?,
        deafen: Boolean,
        spectator: Boolean = false,
        worldUuid: String? = null,
        alternativeIdentity: String? = null
    ) : this(
        name = name,
        coordinates = Coordinates(x, y, z),
        orientation = Orientation.fromYawPitch(yaw, pitch),
        dimension = dimension?.toApiString(),
        worldUuid = worldUuid,
        deafen = deafen,
        spectator = spectator,
        alternativeIdentity = alternativeIdentity
    )

    /**
     * Constructor for Hytale players with world UUID for isolation.
     */
    constructor(
        name: String,
        x: Double, y: Double, z: Double,
        yaw: Float, pitch: Float,
        dimension: Dimension,
        worldUuid: String,
        deafen: Boolean = false,
        spectator: Boolean = false
    ) : this(
        name = name,
        coordinates = Coordinates(x, y, z),
        orientation = Orientation.fromYawPitch(yaw, pitch),
        dimension = dimension.toApiString(),
        worldUuid = worldUuid,
        deafen = deafen,
        spectator = spectator
    )
}
