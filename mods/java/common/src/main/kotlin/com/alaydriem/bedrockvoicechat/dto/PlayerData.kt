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
    val alternativeIdentity: String? = null,
    @SerializedName("player_uuid")
    val playerUuid: String? = null
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
        alternativeIdentity: String? = null,
        playerUuid: String? = null
    ) : this(
        name = name,
        coordinates = Coordinates(x, y, z),
        orientation = Orientation.fromYawPitch(yaw, pitch),
        dimension = dimension?.toApiString(),
        worldUuid = worldUuid,
        deafen = deafen,
        spectator = spectator,
        alternativeIdentity = alternativeIdentity,
        playerUuid = playerUuid
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
        spectator: Boolean = false,
        playerUuid: String? = null
    ) : this(
        name = name,
        coordinates = Coordinates(x, y, z),
        orientation = Orientation.fromYawPitch(yaw, pitch),
        dimension = dimension.toApiString(),
        worldUuid = worldUuid,
        deafen = deafen,
        spectator = spectator,
        playerUuid = playerUuid
    )

    companion object {
        private const val PHANTOM_COORD = -10000.0

        /**
         * Create a phantom PlayerData for a player who has disconnected.
         * Uses death dimension, extreme coordinates, and spectator=true to guarantee
         * the server's proximity checks reject all audio routing for this player.
         */
        fun disconnected(
            name: String,
            dimension: Dimension,
            worldUuid: String?,
            alternativeIdentity: String? = null,
            playerUuid: String? = null
        ): PlayerData = PlayerData(
            name = name,
            coordinates = Coordinates(PHANTOM_COORD, PHANTOM_COORD, PHANTOM_COORD),
            orientation = Orientation(0f, 0f),
            dimension = dimension.toApiString(),
            worldUuid = worldUuid,
            deafen = false,
            spectator = true,
            alternativeIdentity = alternativeIdentity,
            playerUuid = playerUuid
        )
    }
}
