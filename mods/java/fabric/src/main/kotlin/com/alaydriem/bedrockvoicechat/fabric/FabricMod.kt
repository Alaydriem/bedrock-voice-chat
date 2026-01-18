package com.alaydriem.bedrockvoicechat.fabric

import com.alaydriem.bedrockvoicechat.dto.Payload
import com.alaydriem.bedrockvoicechat.native.PositionSender
import com.alaydriem.bedrockvoicechat.network.HttpRequestHandler
import com.alaydriem.bedrockvoicechat.server.BvcServerManager
import net.fabricmc.api.ModInitializer
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerLifecycleEvents
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerTickEvents
import net.fabricmc.fabric.api.networking.v1.ServerPlayConnectionEvents
import org.slf4j.LoggerFactory

/**
 * Fabric mod entry point for Bedrock Voice Chat.
 */
class FabricMod : ModInitializer {
    private val logger = LoggerFactory.getLogger("Bedrock Voice Chat")

    private val configProvider = FabricConfigProvider()
    private val playerDataProvider = FabricPlayerDataProvider()

    private var embeddedServer: BvcServerManager? = null
    private var positionSender: PositionSender? = null
    private var tickCounter = 0
    private var minimumPlayers = 2

    override fun onInitialize() {
        logger.info("Initializing Bedrock Voice Chat")

        // Load and validate configuration
        val config = configProvider.load()
        if (!config.isValid()) {
            logger.error("Invalid configuration - mod will not track players")
            logger.error("Config validation failed: useEmbeddedServer={}, bvcServer={}, accessToken={}",
                config.useEmbeddedServer,
                if (config.bvcServer.isNullOrBlank()) "MISSING" else "set",
                if (config.accessToken.isNullOrBlank()) "MISSING" else "set")
            return
        }

        minimumPlayers = config.minimumPlayers

        // Initialize embedded server if configured
        if (config.useEmbeddedServer) {
            embeddedServer = BvcServerManager(config, configProvider)
            if (!embeddedServer!!.start()) {
                logger.error("Failed to start embedded server - falling back to disabled state")
                embeddedServer = null
                return
            }

            // For embedded mode, create HTTP handler pointing to localhost
            val embedded = config.embeddedConfig
            val localUrl = "https://127.0.0.1:${embedded?.httpPort ?: 443}"
            val accessToken = config.accessToken ?: java.util.UUID.randomUUID().toString()
            val httpHandler = HttpRequestHandler(localUrl, accessToken)
            positionSender = PositionSender(httpHandler, embeddedServer)

            logger.info("Bedrock Voice Chat using embedded server at {}", localUrl)
        } else {
            // External server mode
            val httpHandler = HttpRequestHandler(config.bvcServer!!, config.accessToken!!)
            positionSender = PositionSender(httpHandler, null)

            logger.info("Bedrock Voice Chat will connect to: {}", config.bvcServer)
        }

        ServerPlayConnectionEvents.JOIN.register { handler, _, _ ->
            playerDataProvider.addPlayer(handler.player)
            logger.debug("Player joined: ${handler.player.name.string}")
        }

        ServerPlayConnectionEvents.DISCONNECT.register { handler, _ ->
            playerDataProvider.removePlayer(handler.player)
            logger.debug("Player disconnected: ${handler.player.name.string}")
        }

        ServerTickEvents.END_SERVER_TICK.register { server ->
            playerDataProvider.server = server

            tickCounter++
            if (tickCounter >= 5) {
                tickCounter = 0
                tick()
            }
        }

        // Stop embedded server on shutdown
        ServerLifecycleEvents.SERVER_STOPPING.register { _ ->
            embeddedServer?.stop()
        }
    }

    private fun tick() {
        val sender = positionSender ?: return
        val players = playerDataProvider.collectPlayers()

        if (players.size < minimumPlayers) {
            return
        }

        val payload = Payload(playerDataProvider.getGameType(), players)
        sender.send(payload)
    }
}
