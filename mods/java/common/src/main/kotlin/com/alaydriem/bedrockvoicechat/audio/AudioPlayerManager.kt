package com.alaydriem.bedrockvoicechat.audio

/**
 * Manages audio playback across different game locations.
 * Implementations handle per-location playback state and cleanup.
 */
interface AudioPlayerManager {

    /**
     * Start playback of an audio file at a specific location.
     *
     * @param audioId The audio file ID
     * @param dimensionId The dimension identifier (e.g., "overworld")
     * @param x X coordinate
     * @param y Y coordinate
     * @param z Z coordinate
     * @param worldUuid The world UUID for isolation
     */
    fun startPlayback(
        audioId: String,
        dimensionId: String,
        x: Double,
        y: Double,
        z: Double,
        worldUuid: String
    )

    /**
     * Stop playback at a specific location.
     *
     * @param locationKey The location key (typically "worldUuid:x:y:z")
     */
    fun stopPlayback(locationKey: String)

    /**
     * Check if playback is active at a location.
     *
     * @param locationKey The location key
     * @return true if playback is active, false otherwise
     */
    fun hasActivePlayback(locationKey: String): Boolean

    /**
     * Generate a location key from world UUID and block coordinates.
     * Format: "worldUuid:x:y:z"
     *
     * @param worldUuid The world UUID
     * @param x Block X coordinate
     * @param y Block Y coordinate
     * @param z Block Z coordinate
     * @return The location key
     */
    fun locationKey(worldUuid: String, x: Int, y: Int, z: Int): String

    /**
     * Clean up all active playbacks and resources.
     */
    fun shutdown()
}
