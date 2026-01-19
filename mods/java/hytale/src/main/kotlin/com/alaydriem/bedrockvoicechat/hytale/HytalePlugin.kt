package com.alaydriem.bedrockvoicechat.hytale

import com.alaydriem.bedrockvoicechat.dto.Payload
import com.alaydriem.bedrockvoicechat.native.PositionSender
import com.alaydriem.bedrockvoicechat.network.HttpRequestHandler
import com.alaydriem.bedrockvoicechat.server.BvcServerManager
import com.hypixel.hytale.server.core.HytaleServer
import com.hypixel.hytale.server.core.event.events.player.PlayerConnectEvent
import com.hypixel.hytale.server.core.event.events.player.PlayerDisconnectEvent
import com.hypixel.hytale.server.core.plugin.JavaPlugin
import com.hypixel.hytale.server.core.plugin.JavaPluginInit
import java.util.concurrent.ScheduledFuture
import java.util.concurrent.TimeUnit
import java.util.logging.Level

/**
 * Hytale plugin entry point for Bedrock Voice Chat.
 */
class HytalePlugin(init: JavaPluginInit) : JavaPlugin(init) {
    private val config = withConfig("BedrockVoiceChatPlugin", BedrockVoiceChatConfig.CODEC)
    private val configProvider = HytaleConfigProvider(config)
    private val playerDataProvider = HytalePlayerDataProvider()

    private var embeddedServer: BvcServerManager? = null
    private var positionSender: PositionSender? = null
    private var tickTask: ScheduledFuture<*>? = null
    private var minimumPlayers = 2

    override fun setup() {
        logger.at(Level.INFO).log("Initializing Bedrock Voice Chat for Hytale")

        // Save config to create file if it doesn't exist (with defaults)
        // BsonUtil.writeDocument() will create parent directories automatically
        config.save()

        // Load config via provider (backed by Hytale's Config<T>)
        val modConfig = configProvider.load()
        if (!modConfig.isValid()) {
            logger.at(Level.SEVERE).log("Invalid configuration. Bedrock Voice Chat will not be enabled.")
            logger.at(Level.SEVERE).log("Config validation failed: useEmbeddedServer=${modConfig.useEmbeddedServer}, " +
                "bvcServer=${if (modConfig.bvcServer.isNullOrBlank()) "MISSING" else "set"}, " +
                "accessToken=${if (modConfig.accessToken.isNullOrBlank()) "MISSING" else "set"}")
            return
        }

        minimumPlayers = modConfig.minimumPlayers

        // Initialize embedded server if configured
        if (modConfig.useEmbeddedServer) {
            embeddedServer = BvcServerManager(modConfig, configProvider)
            if (!embeddedServer!!.start()) {
                logger.at(Level.SEVERE).log("Failed to start embedded server - falling back to disabled state")
                embeddedServer = null
                return
            }

            // Embedded mode: use FFI directly, no HTTP handler needed
            positionSender = PositionSender(null, embeddedServer)

            val embedded = modConfig.embeddedConfig
            logger.at(Level.INFO).log("Bedrock Voice Chat using embedded server (QUIC port: ${embedded?.quicPort ?: 8443})")
        } else {
            // External server mode: use HTTP handler
            val httpHandler = HttpRequestHandler(modConfig.bvcServer!!, modConfig.accessToken!!)
            positionSender = PositionSender(httpHandler, null)

            logger.at(Level.INFO).log("Bedrock Voice Chat will connect to: ${modConfig.bvcServer}")
        }

        // Register player connect/disconnect events
        eventRegistry.register(PlayerConnectEvent::class.java) { event ->
            playerDataProvider.addPlayer(event.playerRef)
        }
        eventRegistry.register(PlayerDisconnectEvent::class.java) { event ->
            playerDataProvider.removePlayer(event.playerRef)
        }

        // Schedule tick task using Hytale's built-in executor
        // ~167ms interval = roughly 5 ticks at 30 TPS (Hytale default)
        tickTask = HytaleServer.SCHEDULED_EXECUTOR.scheduleAtFixedRate(
            { tick() }, 167, 167, TimeUnit.MILLISECONDS
        )
    }

    override fun shutdown() {
        tickTask?.let { task ->
            if (!task.isCancelled) {
                task.cancel(false)
            }
        }
        tickTask = null
        embeddedServer?.stop()
        logger.at(Level.INFO).log("Bedrock Voice Chat disabled")
    }

    private fun tick() {
        try {
            val sender = positionSender ?: return
            val players = playerDataProvider.collectPlayers()

            if (players.size < minimumPlayers) {
                return
            }

            val payload = Payload(playerDataProvider.getGameType(), players)
            sender.send(payload)
        } catch (e: Exception) {
            logger.at(Level.WARNING).log("Error during tick: ${e.message}")
        }
    }
}
