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
    val playerUuid: String? = null,
    /**
     * The world this server declares to its BVC peer.
     *
     * Distinct from [worldUuid], which is per-dimension and separates players
     * within this server. This one is per-server and decides which peer link
     * carries them. Absent means the player cannot be scoped to a relay world at
     * all, and the peer boundary refuses their audio rather than guessing.
     */
    @SerializedName("relay_world_uuid")
    val relayWorldUuid: String? = null,
    /**
     * Whether this player holds a voice connection with a bridge on this server rather
     * than with BVC directly.
     *
     * The BVC server counts voice connections it terminates itself, so a bridged player
     * reads as being in the world with no voice at all — and every BVC client beside them
     * is told they cannot hear you, while they can. Only this mod can answer, so it says
     * so here rather than leaving the server to guess from traffic.
     */
    @SerializedName("bridged_voice")
    val bridgedVoice: Boolean = false
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
        playerUuid: String? = null,
        relayWorldUuid: String? = null,
        bridgedVoice: Boolean = false
    ) : this(
        name = name,
        coordinates = Coordinates(x, y, z),
        orientation = Orientation.fromYawPitch(yaw, pitch),
        dimension = dimension?.toApiString(),
        worldUuid = worldUuid,
        deafen = deafen,
        spectator = spectator,
        alternativeIdentity = alternativeIdentity,
        playerUuid = playerUuid,
        relayWorldUuid = relayWorldUuid,
        bridgedVoice = bridgedVoice
    )

    /**
     * Constructor for players whose world UUID is always known, used for world isolation.
     */
    constructor(
        name: String,
        x: Double, y: Double, z: Double,
        yaw: Float, pitch: Float,
        dimension: Dimension,
        worldUuid: String,
        deafen: Boolean = false,
        spectator: Boolean = false,
        playerUuid: String? = null,
        relayWorldUuid: String? = null,
        bridgedVoice: Boolean = false
    ) : this(
        name = name,
        coordinates = Coordinates(x, y, z),
        orientation = Orientation.fromYawPitch(yaw, pitch),
        dimension = dimension.toApiString(),
        worldUuid = worldUuid,
        deafen = deafen,
        spectator = spectator,
        playerUuid = playerUuid,
        relayWorldUuid = relayWorldUuid,
        bridgedVoice = bridgedVoice
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
            playerUuid: String? = null,
            relayWorldUuid: String? = null
        ): PlayerData = PlayerData(
            name = name,
            coordinates = Coordinates(PHANTOM_COORD, PHANTOM_COORD, PHANTOM_COORD),
            orientation = Orientation(0f, 0f),
            dimension = dimension.toApiString(),
            worldUuid = worldUuid,
            deafen = false,
            spectator = true,
            alternativeIdentity = alternativeIdentity,
            playerUuid = playerUuid,
            relayWorldUuid = relayWorldUuid
        )
    }
}
