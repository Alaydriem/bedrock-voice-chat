package com.alaydriem.bedrockvoicechat.integration

import org.slf4j.LoggerFactory
import java.util.UUID
import java.util.concurrent.atomic.AtomicReference

/**
 * Optional integration with the Floodgate API for detecting Bedrock players on Geyser servers.
 * Lazily resolves the API on first use because mod load order (especially on Fabric) can place
 * BVC initialization before Floodgate has registered its API singleton.
 */
class FloodgateIntegration {
    private val log = LoggerFactory.getLogger("BedrockVoiceChat.Floodgate")

    private enum class State { UNRESOLVED, AVAILABLE, ABSENT }
    private val state = AtomicReference(State.UNRESOLVED)
    @Volatile private var apiInstance: Any? = null

    val isAvailable: Boolean get() = resolveApi() != null

    private fun resolveApi(): Any? {
        when (state.get()) {
            State.AVAILABLE -> return apiInstance
            State.ABSENT -> return null
            State.UNRESOLVED -> Unit
        }
        try {
            val clazz = Class.forName("org.geysermc.floodgate.api.FloodgateApi")
            val api = clazz.getMethod("getInstance").invoke(null)
            if (api != null) {
                apiInstance = api
                state.set(State.AVAILABLE)
                log.info("Floodgate API loaded: impl={}", api.javaClass.name)
                return api
            }
            return null
        } catch (e: ClassNotFoundException) {
            state.set(State.ABSENT)
            log.info("Floodgate API not found on classpath — prefix-strip path disabled")
            return null
        } catch (e: Exception) {
            log.warn("Failed to load Floodgate API: {}", e.toString())
            return null
        }
    }

    fun getXboxGamertag(playerUuid: UUID): String? {
        val api = resolveApi() ?: return null
        try {
            val apiClass = api.javaClass
            val isFloodgate = apiClass.getMethod("isFloodgatePlayer", UUID::class.java)
            val floodgateResult = isFloodgate.invoke(api, playerUuid)
            if (floodgateResult != true) {
                log.info("isFloodgatePlayer({}) returned {} — not stripping prefix", playerUuid, floodgateResult)
                return null
            }

            val getPlayer = apiClass.getMethod("getPlayer", UUID::class.java)
            val floodgatePlayer = getPlayer.invoke(api, playerUuid)
            if (floodgatePlayer == null) {
                log.info("getPlayer({}) returned null despite isFloodgatePlayer=true", playerUuid)
                return null
            }

            val playerClass = floodgatePlayer.javaClass
            // getUsername() returns the raw Bedrock gamertag (no prefix);
            // getJavaUsername() / getCorrectUsername() return the Java-visible name (with prefix)
            return playerClass.getMethod("getUsername").invoke(floodgatePlayer) as? String
        } catch (e: Exception) {
            log.warn("Floodgate API call failed for {}: {}", playerUuid, e.toString())
            return null
        }
    }
}
