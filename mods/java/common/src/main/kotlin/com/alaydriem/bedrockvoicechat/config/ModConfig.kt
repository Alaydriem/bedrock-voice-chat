package com.alaydriem.bedrockvoicechat.config

import com.google.gson.annotations.SerializedName

/**
 * Configuration for the BVC mod, shared across all platforms.
 */
class ModConfig {
    @SerializedName(value = "bvc-server", alternate = ["bvcServer"])
    var bvcServer: String? = null

    @SerializedName(value = "access-token", alternate = ["accessToken"])
    var accessToken: String? = null

    @SerializedName(value = "minimum-players", alternate = ["minimumPlayers"])
    var minimumPlayers: Int = 2

    /**
     * Check if the configuration is valid (has required fields set).
     */
    fun isValid(): Boolean =
        !bvcServer.isNullOrBlank() && !accessToken.isNullOrBlank()
}
