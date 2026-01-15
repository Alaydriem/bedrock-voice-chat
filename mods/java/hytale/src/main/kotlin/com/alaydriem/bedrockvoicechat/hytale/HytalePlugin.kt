package com.alaydriem.bedrockvoicechat.hytale

import com.alaydriem.bedrockvoicechat.dto.Payload
import com.alaydriem.bedrockvoicechat.network.HttpRequestHandler
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
    private val config = withConfig("bedrock-voice-chat", HytaleModConfig.CODEC)
    private val configProvider = HytaleConfigProvider(config)
    private val playerDataProvider = HytalePlayerDataProvider()

    private var httpHandler: HttpRequestHandler? = null
    private var tickTask: ScheduledFuture<*>? = null
    private var minimumPlayers = 2

    override fun setup() {
        logger.at(Level.INFO).log("Initializing Bedrock Voice Chat for Hytale")

        // Load config via provider (backed by Hytale's Config<T>)
        val modConfig = configProvider.load()
        if (!modConfig.isValid()) {
            logger.at(Level.SEVERE).log("Invalid configuration - plugin will not track players")
            return
        }

        minimumPlayers = modConfig.minimumPlayers
        httpHandler = HttpRequestHandler(modConfig.bvcServer!!, modConfig.accessToken!!)

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

        logger.at(Level.INFO).log("Bedrock Voice Chat will connect to: ${modConfig.bvcServer}")
    }

    override fun shutdown() {
        tickTask?.let { task ->
            if (!task.isCancelled) {
                task.cancel(false)
            }
        }
    }

    private fun tick() {
        try {
            val handler = httpHandler ?: return
            val players = playerDataProvider.collectPlayers()

            if (players.size < maxOf(minimumPlayers, 2)) {
                return
            }

            val payload = Payload(playerDataProvider.getGameType(), players)
            handler.sendAsync(payload)
        } catch (e: Exception) {
            logger.at(Level.WARNING).log("Error during tick: ${e.message}")
        }
    }
}
