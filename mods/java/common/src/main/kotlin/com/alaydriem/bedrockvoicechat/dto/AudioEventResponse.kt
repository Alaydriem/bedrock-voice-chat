package com.alaydriem.bedrockvoicechat.dto

/**
 * Response from a successful audio play request.
 */
data class AudioEventResponse(
    val event_id: String,
    val duration_ms: Long
)
