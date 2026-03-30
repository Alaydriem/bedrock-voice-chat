package com.alaydriem.bedrockvoicechat.audio.dto

import com.google.gson.annotations.SerializedName

data class AudioEventResponse(
    @SerializedName("event_id")
    val eventId: String,
    @SerializedName("duration_ms")
    val durationMs: Long
)
