package com.alaydriem.bedrockvoicechat.fabric

import com.alaydriem.bedrockvoicechat.dto.Payload
import com.alaydriem.bedrockvoicechat.network.HttpRequestHandler
import net.fabricmc.api.ModInitializer
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

    private var httpHandler: HttpRequestHandler? = null
    private var tickCounter = 0
    private var minimumPlayers = 2

    override fun onInitialize() {
        logger.info("Initializing Bedrock Voice Chat")

        // Load and validate configuration
        val config = configProvider.load()
        if (!config.isValid()) {
            logger.error("Invalid configuration - mod will not track players")
            logger.error("Config validation failed: bvcServer={}, accessToken={}",
                if (config.bvcServer.isNullOrBlank()) "MISSING" else "set",
                if (config.accessToken.isNullOrBlank()) "MISSING" else "set")
            return
        }

        minimumPlayers = config.minimumPlayers
        httpHandler = HttpRequestHandler(config.bvcServer!!, config.accessToken!!)

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

        logger.info("Bedrock Voice Chat will connect to: {}", config.bvcServer)
    }

    private fun tick() {
        val handler = httpHandler ?: return
        val players = playerDataProvider.collectPlayers()

        if (players.size < minimumPlayers) {
            return
        }

        val payload = Payload(playerDataProvider.getGameType(), players)
        handler.sendAsync(payload)
    }
}
