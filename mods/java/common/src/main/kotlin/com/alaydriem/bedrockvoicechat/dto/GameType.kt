package com.alaydriem.bedrockvoicechat.dto

import com.google.gson.annotations.SerializedName

/**
 * Supported game platforms for voice chat integration.
 */
enum class GameType(val value: String) {
    @SerializedName("minecraft")
    MINECRAFT("minecraft"),

    @SerializedName("hytale")
    HYTALE("hytale")
}
