package com.alaydriem.bedrockvoicechat.powernukkitx

import com.alaydriem.bedrockvoicechat.admin.AdminConsole
import com.alaydriem.bedrockvoicechat.audio.AudioEventSender
import com.alaydriem.bedrockvoicechat.audio.JukeboxPlaybackService
import com.alaydriem.bedrockvoicechat.chat.AsyncChatTransport
import com.alaydriem.bedrockvoicechat.chat.ChatChannel
import com.alaydriem.bedrockvoicechat.chat.ChatTransport
import com.alaydriem.bedrockvoicechat.chat.FfiChatTransport
import com.alaydriem.bedrockvoicechat.config.ModConfig
import com.alaydriem.bedrockvoicechat.control.ControlSender
import com.alaydriem.bedrockvoicechat.dto.Payload
import com.alaydriem.bedrockvoicechat.dto.PlayerData
import com.alaydriem.bedrockvoicechat.native.BvcNative
import com.alaydriem.bedrockvoicechat.native.HostCapabilityCheck
import com.alaydriem.bedrockvoicechat.native.HostCapabilitySender
import com.alaydriem.bedrockvoicechat.native.HttpLibraryFetcher
import com.alaydriem.bedrockvoicechat.native.NativeLibraryProvider
import com.alaydriem.bedrockvoicechat.native.NativeManifest
import com.alaydriem.bedrockvoicechat.native.PositionSender
import com.alaydriem.bedrockvoicechat.network.HttpRequestHandler
import com.alaydriem.bedrockvoicechat.powernukkitx.audio.HopperJukeboxRule
import com.alaydriem.bedrockvoicechat.powernukkitx.audio.PnxHopperTask
import com.alaydriem.bedrockvoicechat.powernukkitx.audio.PnxJukeboxListener
import com.alaydriem.bedrockvoicechat.powernukkitx.audio.PnxJukeboxPlayback
import com.alaydriem.bedrockvoicechat.powernukkitx.audio.PnxJukeboxRegistry
import com.alaydriem.bedrockvoicechat.powernukkitx.audio.PnxMainThreadScheduler
import com.alaydriem.bedrockvoicechat.powernukkitx.chat.PnxChatListener
import com.alaydriem.bedrockvoicechat.powernukkitx.chat.PnxMessageTranslator
import com.alaydriem.bedrockvoicechat.powernukkitx.commands.BvcCommand
import com.alaydriem.bedrockvoicechat.server.BvcServerManager
import com.alaydriem.bedrockvoicechat.svc.RelayWorld
import org.powernukkitx.event.EventHandler
import org.powernukkitx.event.Listener
import org.powernukkitx.event.player.PlayerDeathEvent
import org.powernukkitx.event.player.PlayerJoinEvent
import org.powernukkitx.event.player.PlayerQuitEvent
import org.powernukkitx.event.player.PlayerRespawnEvent
import org.powernukkitx.plugin.PluginBase
import org.powernukkitx.scheduler.TaskHandler

/**
 * PowerNukkitX plugin entry point for Bedrock Voice Chat.
 */
class PnxPlugin : PluginBase(), Listener {
    private val configProvider = PnxConfigProvider(this)
    private lateinit var playerDataProvider: PnxPlayerDataProvider
    private lateinit var worldIdentity: PnxWorldIdentity

    private var embeddedServer: BvcServerManager? = null
    private var positionSender: PositionSender? = null
    private var controlSender: ControlSender? = null
    private var chatChannel: ChatTransport? = null
    private var chatSocket: ChatChannel? = null
    private var tickTask: TaskHandler? = null
    private var audioEventSender: AudioEventSender? = null
    private var jukeboxPlayback: JukeboxPlaybackService? = null
    private var hopperTask: TaskHandler? = null
    private var minimumPlayers = 1

    override fun onEnable() {
        getLogger().info("Initializing Bedrock Voice Chat")

        configProvider.createDefaultIfMissing()

        val config = configProvider.load()
        if (!config.isValid()) {
            getLogger().critical("Invalid configuration - plugin will not track players")
            getLogger().critical("Config validation failed: useEmbeddedServer=${config.useEmbeddedServer}, " +
                "bvcServer=${if (config.bvcServer.isNullOrBlank()) "MISSING" else "set"}, " +
                "accessToken=${if (config.accessToken.isNullOrBlank()) "MISSING" else "set"}")
            return
        }

        minimumPlayers = config.minimumPlayers
        val dataFolder = getDataFolder()
        worldIdentity = PnxWorldIdentity(dataFolder)
        val relayWorld = RelayWorld(dataFolder)
        playerDataProvider = PnxPlayerDataProvider(
            worldIdentity = worldIdentity,
            relayWorld = relayWorld,
            playerLookup = { id -> getServer().getPlayer(id).orElse(null) }
        )

        // Native libraries are resolved from the plugin data directory rather than
        // unpacked from the jar. Configured before anything can reach an FFI call,
        // which for the embedded server is its start below.
        BvcNative.configure(
            NativeLibraryProvider(
                cacheRoot = dataFolder,
                manifest = NativeManifest.fromResources(),
                fetcher = HttpLibraryFetcher()
            )
        )

        var httpHandler: HttpRequestHandler? = null
        if (config.useEmbeddedServer) {
            val embedded = BvcServerManager(config, configProvider, "powernukkitx")
            if (!embedded.start()) {
                getLogger().critical("Failed to start embedded server - falling back to disabled state")
                return
            }
            embeddedServer = embedded

            // Embedded mode calls the server over FFI, with no HTTP handler.
            positionSender = PositionSender(null, embedded)
            controlSender = ControlSender(null, embedded)
            audioEventSender = AudioEventSender(null, embedded)

            val quicPort = embedded.effectiveConfig()?.server?.quicPort
            getLogger().info("Bedrock Voice Chat using embedded server (QUIC port: $quicPort)")
        } else {
            val handler = HttpRequestHandler(config.bvcServer!!, config.accessToken!!)
            httpHandler = handler
            positionSender = PositionSender(handler, null)
            controlSender = ControlSender(handler, null)
            audioEventSender = AudioEventSender(handler, null)

            getLogger().info("Bedrock Voice Chat will connect to: ${config.bvcServer}")
        }

        // Every 5 ticks: 250 ms at 20 TPS.
        tickTask = getServer().getScheduler().scheduleRepeatingTask(this, Runnable { tick() }, 5)

        getServer().getPluginManager().registerEvents(this, this)

        startJukebox(audioEventSender!!, relayWorld)
        startChatChannel(config)
        registerCommand()

        reportHostCapability(config, httpHandler)
    }

    /**
     * Plays BVC discs in vanilla jukeboxes, inserted by hand or fed by hoppers.
     */
    private fun startJukebox(sender: AudioEventSender, relayWorld: RelayWorld) {
        val service = JukeboxPlaybackService(sender, PnxMainThreadScheduler(this)) { relayWorld.id() }
        jukeboxPlayback = service

        val playback = PnxJukeboxPlayback(service) { level -> worldIdentity.uuidFor(level.getFolderName()) }
        val registry = PnxJukeboxRegistry()
        val pluginManager = getServer().getPluginManager()
        pluginManager.registerEvents(registry, this)
        pluginManager.registerEvents(PnxJukeboxListener(playback, registry), this)
        registry.seed(getServer())

        hopperTask = getServer().getScheduler().scheduleRepeatingTask(
            this, PnxHopperTask(registry, playback, HopperJukeboxRule()), PnxHopperTask.PERIOD_TICKS
        )
    }

    /**
     * Registers `/bvc` in code rather than in `plugin.yml`. A `commands:` entry would take
     * the `bvc` label first and leave this command reachable only as `bedrockvoicechat:bvc`.
     */
    private fun registerCommand() {
        val sender = controlSender ?: return
        val command = BvcCommand(
            controlSender = sender,
            adminConsole = AdminConsole(playerDataProvider.getGameType()) { embeddedServer },
            canonicalName = playerDataProvider::resolveCanonicalName,
            onlinePlayers = { getServer().getOnlinePlayers().values },
            onServerThread = { task -> getServer().getScheduler().scheduleTask(this, task) },
        )
        if (!getServer().getCommandMap().register("bedrockvoicechat", command)) {
            getLogger().warning("/bvc is taken by another plugin; use /bedrockvoicechat:bvc")
        }
    }

    /**
     * Opens the chat relay to the BVC server.
     *
     * Each level has its own world UUID and chat is server-wide, so every level's id is
     * declared. The default level supplies the canonical id and the name the app's picker
     * shows.
     */
    private fun startChatChannel(config: ModConfig) {
        val defaultLevel = getServer().getDefaultLevel()
        if (defaultLevel == null) {
            getLogger().warning("Bedrock Voice Chat chat relay not started (no levels loaded)")
            return
        }
        val levels = listOf(defaultLevel) + getServer().getLevels().values.filter { it !== defaultLevel }
        val worlds = levels.map { worldIdentity.uuidFor(it.getFolderName()) }.distinct()
        val worldName = defaultLevel.getName()

        var listener: PnxChatListener? = null
        // Broadcasting has to happen on the main thread; both transports deliver on their own.
        val onSay: (String, String) -> Unit = { author, text ->
            getServer().getScheduler().scheduleTask(this, Runnable { listener?.say(author, text) })
        }

        // Embedded shares this process, so it calls into the server rather than dialling a
        // socket back into its own address space.
        val base: ChatTransport = if (config.useEmbeddedServer) {
            val embedded = embeddedServer
            if (embedded == null) {
                getLogger().warning("Bedrock Voice Chat chat relay not started (embedded server not running)")
                return
            }
            FfiChatTransport(embedded, worlds.first(), worldName, worlds, onSay)
        } else {
            val serverUrl = config.bvcServer
            val token = config.accessToken
            if (serverUrl == null || token == null) {
                getLogger().info("Bedrock Voice Chat chat relay not started (no server configured)")
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

        // Every listener reports from the main thread, and neither transport is safe to
        // block it with.
        val transport = AsyncChatTransport(base)
        val language = getServer().getLanguage()
        val translator = PnxMessageTranslator { key, params -> language.tr(key, *params) }

        listener = PnxChatListener(transport, translator, getServer())
        getServer().getPluginManager().registerEvents(listener, this)

        chatChannel = transport
        transport.start()
    }

    override fun onDisable() {
        chatChannel?.stop()
        chatChannel = null
        tickTask?.cancel()
        tickTask = null
        hopperTask?.cancel()
        hopperTask = null
        jukeboxPlayback?.shutdown()
        jukeboxPlayback = null
        embeddedServer?.stop()
        embeddedServer = null
        getLogger().info("Bedrock Voice Chat disabled")
    }

    /**
     * Measures whether this host could run the skinny jar, on a daemon thread so it
     * never delays startup and its result never affects the run.
     *
     * Gated by the operator's own `telemetry` key, and also by the embedded server's
     * resolved setting when there is one. Off means no request is made at all.
     */
    private fun reportHostCapability(config: ModConfig, httpHandler: HttpRequestHandler?) {
        val embedded = embeddedServer
        val permitted = config.telemetry && (embedded == null || embedded.telemetryEnabled)
        if (!permitted) {
            return
        }

        val dataFolder = getDataFolder()
        Thread({
            try {
                val manifest = NativeManifest.fromResources()
                val provider = NativeLibraryProvider(
                    cacheRoot = dataFolder,
                    manifest = manifest,
                    fetcher = HttpLibraryFetcher()
                )
                val report = HostCapabilityCheck(
                    provider, manifest, HttpLibraryFetcher(), manifest.release, true,
                    loader = "powernukkitx", mode = if (embedded != null) "embedded" else "external"
                ).run()
                report?.let { HostCapabilitySender(httpHandler, embedded).send(it) }
            } catch (e: Exception) {
                getLogger().debug("Host capability check did not complete: ${e.message}")
            }
        }, "bvc-host-capability").apply { isDaemon = true }.start()
    }

    private fun tick() {
        val sender = positionSender ?: return
        val players = playerDataProvider.collectPlayers()

        if (players.size < minimumPlayers) {
            return
        }

        sender.send(Payload(playerDataProvider.getGameType(), players))
    }

    @EventHandler
    fun onPlayerJoin(event: PlayerJoinEvent) {
        playerDataProvider.addPlayer(event.getPlayer())
    }

    @EventHandler
    fun onPlayerQuit(event: PlayerQuitEvent) {
        val player = event.getPlayer()
        val sender = positionSender
        val name = playerDataProvider.resolveCanonicalName(player)

        // Captured before removePlayer, which clears this player's state.
        val phantom = if (sender != null) {
            PlayerData.disconnected(
                name = name,
                worldUuid = playerDataProvider.worldUuidOf(player),
                playerUuid = player.getUniqueId().toString()
            )
        } else null

        playerDataProvider.removePlayer(player)

        if (phantom != null && sender != null) {
            sender.send(Payload(playerDataProvider.getGameType(), listOf(phantom)))
            getLogger().info("Sent disconnect phantom for player: $name")
        }
    }

    @EventHandler
    fun onPlayerDeath(event: PlayerDeathEvent) {
        playerDataProvider.markDead(event.getEntity())
    }

    @EventHandler
    fun onPlayerRespawn(event: PlayerRespawnEvent) {
        playerDataProvider.markAlive(event.getPlayer())
    }
}
