package com.alaydriem.bedrockvoicechat.paper

import com.alaydriem.bedrockvoicechat.dto.Payload
import com.alaydriem.bedrockvoicechat.network.HttpRequestHandler
import org.bukkit.event.EventHandler
import org.bukkit.event.Listener
import org.bukkit.event.player.PlayerJoinEvent
import org.bukkit.event.player.PlayerQuitEvent
import org.bukkit.plugin.java.JavaPlugin
import org.bukkit.scheduler.BukkitTask

/**
 * Paper plugin entry point for Bedrock Voice Chat.
 * Implements Listener for event-driven player tracking (like Hytale).
 */
class PaperPlugin : JavaPlugin(), Listener {
    private val configProvider = PaperConfigProvider(this)
    private val playerDataProvider = PaperPlayerDataProvider()

    private var httpHandler: HttpRequestHandler? = null
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
            logger.severe("Please configure bvc-server and access-token in config.yml")
            return
        }

        minimumPlayers = config.minimumPlayers
        httpHandler = HttpRequestHandler(config.bvcServer!!, config.accessToken!!)

        // Register this plugin as event listener for player events
        server.pluginManager.registerEvents(this, this)

        // Schedule tick task every 5 ticks (250ms at 20 TPS)
        tickTask = server.scheduler.runTaskTimer(this, Runnable { tick() }, 0L, 5L)

        logger.info("Bedrock Voice Chat will connect to: ${config.bvcServer}")
    }

    override fun onDisable() {
        tickTask?.cancel()
        tickTask = null
        logger.info("Bedrock Voice Chat disabled")
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

    @EventHandler
    fun onPlayerJoin(event: PlayerJoinEvent) {
        playerDataProvider.addPlayer(event.player)
    }

    @EventHandler
    fun onPlayerQuit(event: PlayerQuitEvent) {
        playerDataProvider.removePlayer(event.player)
    }
}
