package com.alaydriem.bedrockvoicechat.dto

/**
 * Player orientation (yaw and pitch).
 * Stored as 'x' (yaw) and 'y' (pitch) for API compatibility.
 */
data class Orientation(
    val x: Float,
    val y: Float
) {
    companion object {
        fun fromYawPitch(yaw: Float, pitch: Float) = Orientation(yaw, pitch)
    }
}
