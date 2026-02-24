package com.alaydriem.bedrockvoicechat.dto

/**
 * Request to start audio playback at specific world coordinates.
 */
data class AudioPlayRequest(
    val audio_file_id: String,
    val coordinates: Coordinates,
    val dimension: String,
    val world_uuid: String
)
