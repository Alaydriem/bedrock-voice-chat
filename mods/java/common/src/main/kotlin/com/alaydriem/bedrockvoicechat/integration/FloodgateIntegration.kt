package com.alaydriem.bedrockvoicechat.integration

import java.util.UUID

/**
 * Optional integration with the Floodgate API for detecting Bedrock players on Geyser servers.
 * Safely handles the case where Floodgate is not installed (API class not found at runtime).
 */
class FloodgateIntegration {
    private val available: Boolean
    private val apiInstance: Any?

    init {
        var api: Any? = null
        var isAvailable = false
        try {
            val clazz = Class.forName("org.geysermc.floodgate.api.FloodgateApi")
            val getInstance = clazz.getMethod("getInstance")
            api = getInstance.invoke(null)
            isAvailable = true
        } catch (_: Exception) {
            // Floodgate not installed
        }
        this.apiInstance = api
        this.available = isAvailable
    }

    val isAvailable: Boolean get() = available

    /**
     * Get the Xbox gamertag for a Floodgate (Bedrock) player.
     * Returns null if Floodgate is not installed or the player is not a Bedrock player.
     */
    fun getXboxGamertag(playerUuid: UUID): String? {
        val api = apiInstance ?: return null
        try {
            val apiClass = api.javaClass
            val isFloodgate = apiClass.getMethod("isFloodgatePlayer", UUID::class.java)
            if (isFloodgate.invoke(api, playerUuid) != true) return null

            val getPlayer = apiClass.getMethod("getPlayer", UUID::class.java)
            val floodgatePlayer = getPlayer.invoke(api, playerUuid) ?: return null

            val playerClass = floodgatePlayer.javaClass
            return try {
                val getUsername = playerClass.getMethod("getUsername")
                getUsername.invoke(floodgatePlayer) as? String
            } catch (_: Exception) {
                null
            }
        } catch (_: Exception) {
            return null
        }
    }
}
