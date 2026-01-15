package com.alaydriem.bedrockvoicechat.config

import com.google.gson.annotations.SerializedName

/**
 * Configuration for the BVC mod, shared across all platforms.
 */
class ModConfig {
    @SerializedName("bvc-server")
    var bvcServer: String? = null

    @SerializedName("access-token")
    var accessToken: String? = null

    @SerializedName("minimum-players")
    var minimumPlayers: Int = 2

    /**
     * Check if the configuration is valid (has required fields set).
     */
    fun isValid(): Boolean =
        !bvcServer.isNullOrBlank() && !accessToken.isNullOrBlank()
}
