package com.alaydriem.bedrockvoicechat.audio

data class AudioPlayerState(
    val audioId: String,
    var eventId: String?,
    var isPlaying: Boolean = false,
    val dimensionId: String,
    val x: Double,
    val y: Double,
    val z: Double,
    val worldUuid: String
)
