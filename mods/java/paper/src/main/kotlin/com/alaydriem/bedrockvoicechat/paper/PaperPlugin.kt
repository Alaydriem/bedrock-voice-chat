package com.alaydriem.bedrockvoicechat.paper

import com.alaydriem.bedrockvoicechat.dto.Payload
import com.alaydriem.bedrockvoicechat.native.PositionSender
import com.alaydriem.bedrockvoicechat.network.HttpRequestHandler
import com.alaydriem.bedrockvoicechat.server.BvcServerManager
import org.bukkit.event.EventHandler
import org.bukkit.event.Listener
import org.bukkit.event.entity.PlayerDeathEvent
import org.bukkit.event.player.PlayerJoinEvent
import org.bukkit.event.player.PlayerQuitEvent
import org.bukkit.event.player.PlayerRespawnEvent
import org.bukkit.plugin.java.JavaPlugin
import org.bukkit.scheduler.BukkitTask

/**
 * Paper plugin entry point for Bedrock Voice Chat.
 * Implements Listener for event-driven player tracking (like Hytale).
 */
class PaperPlugin : JavaPlugin(), Listener {
    private val configProvider = PaperConfigProvider(this)
    private val playerDataProvider = PaperPlayerDataProvider()

    private var embeddedServer: BvcServerManager? = null
    private var positionSender: PositionSender? = null
    private var tickTask: BukkitTask? = null
    private var minimumPlayers = 2

    override fun onEnable() {
        logger.info("Initializing Bedrock Voice Chat")

        // Create default config if missing
        configProvider.createDefaultIfMissing()

        // Load and validate configuration
        val config = configProvider.load()
        if (!config.isValid()) {
            logger.severe("Invalid configuration - plugin will not track players")
            logger.severe("Config validation failed: useEmbeddedServer=${config.useEmbeddedServer}, " +
                "bvcServer=${if (config.bvcServer.isNullOrBlank()) "MISSING" else "set"}, " +
                "accessToken=${if (config.accessToken.isNullOrBlank()) "MISSING" else "set"}")
            return
        }

        minimumPlayers = config.minimumPlayers

        // Initialize embedded server if configured
        if (config.useEmbeddedServer) {
            embeddedServer = BvcServerManager(config, configProvider)
            if (!embeddedServer!!.start()) {
                logger.severe("Failed to start embedded server - falling back to disabled state")
                embeddedServer = null
                return
            }

            // Embedded mode: use FFI directly, no HTTP handler needed
            positionSender = PositionSender(null, embeddedServer)

            val embedded = config.embeddedConfig
            logger.info("Bedrock Voice Chat using embedded server (QUIC port: ${embedded?.quicPort ?: 8443})")
        } else {
            // External server mode: use HTTP handler
            val httpHandler = HttpRequestHandler(config.bvcServer!!, config.accessToken!!)
            positionSender = PositionSender(httpHandler, null)

            logger.info("Bedrock Voice Chat will connect to: ${config.bvcServer}")
        }

        // Set server reference on data provider for player lookups
        playerDataProvider.server = server

        // Register this plugin as event listener for player events
        server.pluginManager.registerEvents(this, this)

        // Schedule tick task every 5 ticks (250ms at 20 TPS)
        tickTask = server.scheduler.runTaskTimer(this, Runnable { tick() }, 0L, 5L)
    }

    override fun onDisable() {
        tickTask?.cancel()
        tickTask = null
        embeddedServer?.stop()
        logger.info("Bedrock Voice Chat disabled")
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

    @EventHandler
    fun onPlayerJoin(event: PlayerJoinEvent) {
        playerDataProvider.addPlayer(event.player)
    }

    @EventHandler
    fun onPlayerQuit(event: PlayerQuitEvent) {
        playerDataProvider.removePlayer(event.player)
    }

    @EventHandler
    fun onPlayerDeath(event: PlayerDeathEvent) {
        playerDataProvider.markDead(event.entity)
    }

    @EventHandler
    fun onPlayerRespawn(event: PlayerRespawnEvent) {
        playerDataProvider.markAlive(event.player)
    }
}
