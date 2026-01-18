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
    val deafen: Boolean
) {
    /**
     * Constructor for Minecraft players (Fabric/Paper).
     * Does not include worldUuid since Minecraft dimensions are global.
     */
    constructor(
        name: String,
        x: Double, y: Double, z: Double,
        yaw: Float, pitch: Float,
        dimension: Dimension?,
        deafen: Boolean
    ) : this(
        name = name,
        coordinates = Coordinates(x, y, z),
        orientation = Orientation(yaw, pitch),
        dimension = dimension?.toApiString(),
        worldUuid = null,
        deafen = deafen
    )

    /**
     * Constructor for Hytale players with world UUID for isolation.
     */
    constructor(
        name: String,
        x: Double, y: Double, z: Double,
        yaw: Float, pitch: Float,
        dimension: Dimension.Hytale,
        worldUuid: String
    ) : this(
        name = name,
        coordinates = Coordinates(x, y, z),
        orientation = Orientation(yaw, pitch),
        dimension = dimension.toApiString(),
        worldUuid = worldUuid,
        deafen = false
    )
}
