package com.alaydriem.bedrockvoicechat.audio.dto

import com.alaydriem.bedrockvoicechat.dto.Coordinates
import com.google.gson.annotations.SerializedName

data class AudioPlayRequest(
    @SerializedName("audio_file_id")
    val audioFileId: String,
    val game: GameAudioRequest
)

data class GameAudioRequest(
    val game: String,
    val coordinates: Coordinates,
    val dimension: String,
    @SerializedName("world_uuid")
    val worldUuid: String,
    /**
     * The world this server declares to its peers.
     *
     * Without it the synthetic speaker the server mints for a playback has no world
     * identifier, and the peer boundary refuses to forward it. The playback then
     * reaches this server's own clients and nothing bridged, which looks like a
     * jukebox problem rather than a missing field.
     */
    @SerializedName("relay_world_uuid")
    val relayWorldUuid: String? = null
)
