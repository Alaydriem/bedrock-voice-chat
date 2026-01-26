package com.alaydriem.bedrockvoicechat.hytale

import com.alaydriem.bedrockvoicechat.dto.Payload
import com.alaydriem.bedrockvoicechat.hytale.systems.CrouchTickingSystem
import com.alaydriem.bedrockvoicechat.hytale.systems.DeathChangeSystem
import com.alaydriem.bedrockvoicechat.hytale.systems.SpectatorChangeSystem
import com.alaydriem.bedrockvoicechat.native.PositionSender
import com.alaydriem.bedrockvoicechat.network.HttpRequestHandler
import com.alaydriem.bedrockvoicechat.server.BvcServerManager
import com.hypixel.hytale.server.core.event.events.player.PlayerConnectEvent
import com.hypixel.hytale.server.core.event.events.player.PlayerDisconnectEvent
import com.hypixel.hytale.server.core.plugin.JavaPlugin
import com.hypixel.hytale.server.core.plugin.JavaPluginInit
import java.util.concurrent.Executors
import java.util.concurrent.ScheduledExecutorService
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
    private var threadPool: ScheduledExecutorService? = null
    private var tickTask: ScheduledFuture<*>? = null
    private var crouchSystem: CrouchTickingSystem? = null
    private var minimumPlayers = 2

    override fun setup() {
        logger.at(Level.INFO).log("Initializing Bedrock Voice Chat for Hytale")

        // Save config to create file if it doesn't exist (with defaults)
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

        // Create our own thread pool for async tick (isolated from Hytale's internal threading)
        threadPool = Executors.newScheduledThreadPool(2) { r ->
            Thread(r).apply {
                name = "BVC-Thread-${threadId()}"
                isDaemon = true
            }
        }

        // Register player connect/disconnect events
        registerEvents()

        // Register ECS systems for component change detection (runs on world thread)
        registerSystems()

        // Schedule tick on our thread pool (reads from cache only, no component access)
        tickTask = threadPool?.scheduleAtFixedRate(
            { tick() }, 167, 167, TimeUnit.MILLISECONDS
        )

        logger.at(Level.INFO).log("Bedrock Voice Chat initialized successfully")
    }

    override fun shutdown() {
        // Cancel tick task
        tickTask?.let { task ->
            if (!task.isCancelled) {
                task.cancel(false)
            }
        }
        tickTask = null

        // Shutdown thread pool gracefully
        threadPool?.shutdown()
        try {
            if (threadPool?.awaitTermination(5, TimeUnit.SECONDS) != true) {
                threadPool?.shutdownNow()
            }
        } catch (e: InterruptedException) {
            threadPool?.shutdownNow()
        }
        threadPool = null

        embeddedServer?.stop()
        logger.at(Level.INFO).log("Bedrock Voice Chat disabled")
    }

    private fun registerEvents() {
        // Player connect/disconnect for tracking online players
        eventRegistry.register(PlayerConnectEvent::class.java) { event ->
            playerDataProvider.addPlayer(event.playerRef)
        }
        eventRegistry.register(PlayerDisconnectEvent::class.java) { event ->
            playerDataProvider.removePlayer(event.playerRef)
            crouchSystem?.removePlayer(event.playerRef.uuid)
        }
    }

    private fun registerSystems() {
        // Death detection via ECS system (fires when DeathComponent is added/removed)
        val deathSystem = DeathChangeSystem(
            { uuid -> playerDataProvider.markDead(uuid) },
            { uuid -> playerDataProvider.markAlive(uuid) }
        )
        entityStoreRegistry.registerSystem(deathSystem)
        logger.at(Level.INFO).log("[BVC] Registered DeathChangeSystem")

        // Crouch detection via tick-based system (polls MovementStatesComponent every tick)
        val crouchSys = CrouchTickingSystem { uuid, crouching ->
            playerDataProvider.setCrouching(uuid, crouching)
        }
        crouchSystem = crouchSys
        entityStoreRegistry.registerSystem(crouchSys)
        logger.at(Level.INFO).log("[BVC] Registered CrouchTickingSystem")

        // Spectator detection via ECS system (fires when HiddenFromAdventurePlayers is added/removed)
        val spectatorSystem = SpectatorChangeSystem { uuid, spectator ->
            playerDataProvider.setSpectator(uuid, spectator)
        }
        entityStoreRegistry.registerSystem(spectatorSystem)
        logger.at(Level.INFO).log("[BVC] Registered SpectatorChangeSystem")
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
