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
    val worldUuid: String
)
