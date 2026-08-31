package com.alaydriem.bedrockvoicechat.config

import com.alaydriem.bedrockvoicechat.config.generated.EmbeddedServerConfig
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
    var minimumPlayers: Int = 1

    /**
     * Whether this mod may report facts about its own host.
     *
     * Owned by the Minecraft operator rather than read from the BVC server, because
     * the request leaves this machine and, in external mode, the BVC server may
     * belong to a hosting provider rather than to the person who installed the mod.
     * In embedded mode the server's own `features.telemetry` applies as well, and
     * either being off means nothing is sent.
     */
    @SerializedName(value = "telemetry", alternate = ["telemetryEnabled"])
    var telemetry: Boolean = true

    /**
     * Formerly the BVC server's peerlink, copied by hand into this file.
     *
     * Read from the server's `/api/config` now, which is unauthenticated and already
     * reachable in external mode. Retained so an existing configuration still loads;
     * the value is ignored.
     */
    @SerializedName(value = "svc-bridge-peerlink", alternate = ["svcBridgePeerlink"])
    var svcBridgePeerlink: String? = null

    // Embedded server mode settings
    @SerializedName(value = "use-embedded-server", alternate = ["useEmbeddedServer"])
    var useEmbeddedServer: Boolean = false

    @SerializedName(value = "embedded-config", alternate = ["embeddedConfig"])
    var embeddedConfig: EmbeddedServerConfig? = null

    /**
     * Legacy embedded-config keys found on load, as `old-key -> new.path`.
     * Populated by the config provider, which is the only place that still sees
     * the raw document.
     */
    @Transient
    var legacyKeys: List<String> = emptyList()

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
