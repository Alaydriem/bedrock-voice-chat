package com.alaydriem.bedrockvoicechat.config

import com.google.gson.annotations.SerializedName

/**
 * Configuration for the BVC mod, shared across all platforms.
 */
class ModConfig {
    // External server mode settings
    @SerializedName(value = "bvc-server", alternate = ["bvcServer"])
    var bvcServer: String? = null

    @SerializedName(value = "access-token", alternate = ["accessToken"])
    var accessToken: String? = null

    @SerializedName(value = "minimum-players", alternate = ["minimumPlayers"])
    var minimumPlayers: Int = 2

    // Embedded server mode settings
    @SerializedName(value = "use-embedded-server", alternate = ["useEmbeddedServer"])
    var useEmbeddedServer: Boolean = false

    @SerializedName(value = "embedded-config", alternate = ["embeddedConfig"])
    var embeddedConfig: EmbeddedConfig? = null

    /**
     * Check if the configuration is valid.
     * For embedded mode, we don't need external server URL.
     * For external mode, we need both server URL and access token.
     */
    fun isValid(): Boolean = when {
        useEmbeddedServer -> true
        else -> !bvcServer.isNullOrBlank() && !accessToken.isNullOrBlank()
    }
}
