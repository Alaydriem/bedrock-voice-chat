package com.alaydriem.bedrockvoicechat.paper

import com.alaydriem.bedrockvoicechat.audio.AudioEventSender
import com.alaydriem.bedrockvoicechat.chat.AsyncChatTransport
import com.alaydriem.bedrockvoicechat.chat.ChatChannel
import com.alaydriem.bedrockvoicechat.chat.ChatTransport
import com.alaydriem.bedrockvoicechat.chat.FfiChatTransport
import com.alaydriem.bedrockvoicechat.config.ModConfig
import com.alaydriem.bedrockvoicechat.control.ControlSender
import com.alaydriem.bedrockvoicechat.paper.chat.PaperChatListener
import com.alaydriem.bedrockvoicechat.dto.Dimension
import com.alaydriem.bedrockvoicechat.dto.Payload
import com.alaydriem.bedrockvoicechat.dto.PlayerData
import com.alaydriem.bedrockvoicechat.native.PositionSender
import com.alaydriem.bedrockvoicechat.network.HttpRequestHandler
import com.alaydriem.bedrockvoicechat.paper.audio.JukeboxListener
import com.alaydriem.bedrockvoicechat.paper.audio.PaperAudioPlayerManager
import com.alaydriem.bedrockvoicechat.paper.commands.ControlCommands
import com.alaydriem.bedrockvoicechat.paper.commands.DiscCommand
import com.alaydriem.bedrockvoicechat.server.BvcServerManager
import io.papermc.paper.command.brigadier.Commands
import io.papermc.paper.plugin.lifecycle.event.types.LifecycleEvents
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
 * Implements Listener for event-driven player tracking.
 */
class PaperPlugin : JavaPlugin(), Listener {
    private val configProvider = PaperConfigProvider(this)
    private lateinit var playerDataProvider: PaperPlayerDataProvider

    private var embeddedServer: BvcServerManager? = null
    private var positionSender: PositionSender? = null
    private var audioEventSender: AudioEventSender? = null
    private var controlSender: ControlSender? = null
    private var chatChannel: ChatTransport? = null
    private var chatSocket: ChatChannel? = null
    private var audioPlayerManager: PaperAudioPlayerManager? = null
    private var tickTask: BukkitTask? = null
    private var minimumPlayers = 1

    @Suppress("UnstableApiUsage")
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
        playerDataProvider = PaperPlayerDataProvider()

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
            audioEventSender = AudioEventSender(null, embeddedServer)
            controlSender = ControlSender(null, embeddedServer)

            val quicPort = embeddedServer?.effectiveConfig()?.server?.quicPort
            logger.info("Bedrock Voice Chat using embedded server (QUIC port: $quicPort)")
        } else {
            // External server mode: use HTTP handler
            val httpHandler = HttpRequestHandler(config.bvcServer!!, config.accessToken!!)
            positionSender = PositionSender(httpHandler, null)
            audioEventSender = AudioEventSender(httpHandler, null)
            controlSender = ControlSender(httpHandler, null)

            logger.info("Bedrock Voice Chat will connect to: ${config.bvcServer}")
        }

        // Set up audio player manager
        val sender = audioEventSender!!
        audioPlayerManager = PaperAudioPlayerManager(sender, this)

        // Set server reference on data provider for player lookups
        playerDataProvider.server = server

        // Register this plugin as event listener for player events
        server.pluginManager.registerEvents(this, this)

        // Register jukebox listener for BVC disc playback
        server.pluginManager.registerEvents(JukeboxListener(audioPlayerManager!!, this), this)

        startChatChannel(config)

        // Register a single /bvc root so disc/give and the control subcommands share
        // one registration rather than relying on the registrar merging duplicate roots.
        val discCommands = DiscCommand(this)
        val ctlCommands =
            controlSender?.let { ControlCommands(it, playerDataProvider::resolveCanonicalName) }
        lifecycleManager.registerEventHandler(LifecycleEvents.COMMANDS) { event ->
            val bvc = Commands.literal("bvc")
            discCommands.addTo(bvc)
            ctlCommands?.addTo(bvc)
            event.registrar().register(bvc.build(), "Bedrock Voice Chat commands")
        }

        // Schedule tick task every 5 ticks (250ms at 20 TPS)
        tickTask = server.scheduler.runTaskTimer(this, Runnable { tick() }, 0L, 5L)
    }

    /**
     * Opens the chat relay to the BVC server.
     *
     * Paper mints a world UUID per dimension and chat is server-wide, so every world's id is
     * declared: a line typed in the overworld has to reach somebody standing in the nether.
     * The primary world supplies the canonical id and the name the app's picker shows.
     */
    private fun startChatChannel(config: ModConfig) {
        val worlds = server.worlds.map { it.uid.toString() }
        if (worlds.isEmpty()) {
            logger.warning("Bedrock Voice Chat chat relay not started (no worlds loaded)")
            return
        }
        val worldName = server.worlds.first().name

        var listener: PaperChatListener? = null
        // Broadcasting has to happen on the main thread; both transports deliver on their own.
        val onSay: (String, String) -> Unit = { author, text ->
            server.scheduler.runTask(this, Runnable { listener?.say(author, text) })
        }

        // Chosen by mode, exactly as ControlSender and AudioEventSender already are. Embedded
        // shares this process, so it calls into the server rather than dialling a socket back
        // into its own address space.
        val base: ChatTransport = if (config.useEmbeddedServer) {
            val embedded = embeddedServer
            if (embedded == null) {
                logger.warning("Bedrock Voice Chat chat relay not started (embedded server not running)")
                return
            }
            FfiChatTransport(embedded, worlds.first(), worldName, worlds, onSay)
        } else {
            val serverUrl = config.bvcServer
            val token = config.accessToken
            if (serverUrl == null || token == null) {
                logger.info("Bedrock Voice Chat chat relay not started (no server configured)")
                return
            }
            val socket = ChatChannel(
                serverUrl = serverUrl,
                accessToken = token,
                worldUuid = worlds.first(),
                worldName = worldName,
                worlds = worlds,
                onSay = onSay,
                send = { body -> chatSocket?.sendOverSocket(body) }
            )
            chatSocket = socket
            socket
        }

        // Every listener below reports from the main thread, and neither transport is safe to
        // block it with.
        val transport = AsyncChatTransport(base)

        listener = PaperChatListener(transport)
        server.pluginManager.registerEvents(listener, this)

        chatChannel = transport
        transport.start()
    }

    override fun onDisable() {
        chatChannel?.stop()
        chatChannel = null
        tickTask?.cancel()
        tickTask = null
        audioPlayerManager?.shutdown()
        audioPlayerManager = null
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
        val player = event.player
        val sender = positionSender
        val canonicalName = playerDataProvider.resolveCanonicalName(player)

        // Capture phantom data before removePlayer() clears state
        val phantom = if (sender != null) {
            PlayerData.disconnected(
                name = canonicalName,
                dimension = Dimension.Minecraft.DEATH,
                worldUuid = player.location.world?.uid?.toString(),
                playerUuid = player.uniqueId.toString()
            )
        } else null

        playerDataProvider.removePlayer(player)

        if (phantom != null && sender != null) {
            val payload = Payload(playerDataProvider.getGameType(), listOf(phantom))
            sender.send(payload)
            logger.info("Sent disconnect phantom for player: $canonicalName")
        }
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
