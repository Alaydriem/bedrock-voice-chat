package com.alaydriem.bedrockvoicechat.dto

/**
 * Player orientation.
 * x = pitch (up/down, ±90°), y = yaw (facing direction, ±180°)
 */
data class Orientation(
    val x: Float,
    val y: Float
) {
    companion object {
        fun fromYawPitch(yaw: Float, pitch: Float) = Orientation(x = pitch, y = yaw)
    }
}
