package com.alaydriem.bedrockvoicechat.dto

/**
 * Request to stop an active audio playback event.
 */
data class AudioStopRequest(
    val event_id: String
)
