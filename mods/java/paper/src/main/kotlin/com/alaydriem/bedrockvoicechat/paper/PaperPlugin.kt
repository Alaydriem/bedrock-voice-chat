package com.alaydriem.bedrockvoicechat.paper

import com.alaydriem.bedrockvoicechat.audio.AudioEventSender
import com.alaydriem.bedrockvoicechat.chat.AsyncChatTransport
import com.alaydriem.bedrockvoicechat.chat.ChatChannel
import com.alaydriem.bedrockvoicechat.chat.ChatTransport
import com.alaydriem.bedrockvoicechat.chat.FfiChatTransport
import com.alaydriem.bedrockvoicechat.config.ModConfig
import com.alaydriem.bedrockvoicechat.config.generated.EmbeddedServerConfig
import com.alaydriem.bedrockvoicechat.control.ControlSender
import com.alaydriem.bedrockvoicechat.paper.chat.PaperChatListener
import com.alaydriem.bedrockvoicechat.dto.Dimension
import com.alaydriem.bedrockvoicechat.dto.Payload
import com.alaydriem.bedrockvoicechat.dto.PlayerData
import com.alaydriem.bedrockvoicechat.native.BvcNative
import com.alaydriem.bedrockvoicechat.native.HostCapabilityCheck
import com.alaydriem.bedrockvoicechat.native.HostCapabilitySender
import com.alaydriem.bedrockvoicechat.native.HttpLibraryFetcher
import com.alaydriem.bedrockvoicechat.native.NativeLibraryProvider
import com.alaydriem.bedrockvoicechat.native.NativeManifest
import com.alaydriem.bedrockvoicechat.native.PositionSender
import com.alaydriem.bedrockvoicechat.paper.svc.PaperSvcChannelFactory
import com.alaydriem.bedrockvoicechat.paper.svc.PaperSvcWiring
import com.alaydriem.bedrockvoicechat.svc.BridgePeering
import com.alaydriem.bedrockvoicechat.paper.commands.PeerCommand
import com.alaydriem.bedrockvoicechat.svc.EmbeddedGrant
import com.alaydriem.bedrockvoicechat.svc.PairingRequest
import com.alaydriem.bedrockvoicechat.svc.PeeringEligibility
import com.alaydriem.bedrockvoicechat.svc.SvcPairing
import com.alaydriem.bedrockvoicechat.svc.LiveClients
import com.alaydriem.bedrockvoicechat.svc.RelayWorld
import com.alaydriem.bedrockvoicechat.svc.SvcAvailability
import com.alaydriem.bedrockvoicechat.svc.SvcBridgeHost
import de.maxhenkel.voicechat.api.BukkitVoicechatService
import java.io.File
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
    private var relayWorld: RelayWorld? = null

    // Held so `/bvc peer` and the bridge's own peerlink lookup share one answer, and one
    // fetch of `/api/config`.
    private var pairingEligibility: PeeringEligibility? = null
    private var svcBridgeHost: SvcBridgeHost? = null

    // One instance: it caches its answer and logs on first resolve, so a second
    // instance reports the same detection a second time.
    private val svcAvailability = SvcAvailability()
    private var positionSender: PositionSender? = null
    private var audioEventSender: AudioEventSender? = null
    private var controlSender: ControlSender? = null
    private var chatChannel: ChatTransport? = null
    private var chatSocket: ChatChannel? = null
    private var audioPlayerManager: PaperAudioPlayerManager? = null
    private var tickTask: BukkitTask? = null
    private var minimumPlayers = 1

    @Suppress("UnstableApiUsage")
    /**
     * Measures whether this host could run the skinny jar, on a daemon thread so it
     * never delays startup and its result never affects the run.
     *
     * Gated by the operator's own `telemetry` key, and additionally by the embedded
     * server's resolved setting when there is one. Off means no request is made at
     * all, rather than one made and discarded.
     */
    private fun reportHostCapability(config: ModConfig, httpHandler: HttpRequestHandler?) {
        val embedded = embeddedServer
        val permitted = config.telemetry && (embedded == null || embedded.telemetryEnabled)
        if (!permitted) {
            return
        }

        Thread({
            try {
                val manifest = NativeManifest.fromResources()
                val provider = NativeLibraryProvider(
                    cacheRoot = dataFolder,
                    manifest = manifest,
                    fetcher = HttpLibraryFetcher()
                )
                val report = HostCapabilityCheck(
                    provider, manifest, HttpLibraryFetcher(), manifest.release, true
                ).run()
                report?.let { HostCapabilitySender(httpHandler, embedded).send(it) }
            } catch (e: Exception) {
                logger.fine("Host capability check did not complete: ${e.message}")
            }
        }, "bvc-host-capability").apply { isDaemon = true }.start()
    }

    /**
     * Bridges Simple Voice Chat when it is present, and does nothing when it is not.
     *
     * The availability check runs before any bridge class is touched, so a server
     * without SVC never loads one — those classes name SVC types and would fail to
     * link.
     */
    private fun startSvcBridge(config: ModConfig) {
        if (!svcAvailability.isAvailable) {
            return
        }

        val service = server.servicesManager.load(BukkitVoicechatService::class.java)
        if (service == null) {
            logger.warning("Simple Voice Chat is present but its service is not registered")
            return
        }

        val world = relayWorld ?: return
        val nodeDir = File(dataFolder, "svc-bridge")
        val wiring = PaperSvcWiring(server, playerDataProvider)

        // Embedded owns both sides, so the server it is about to start has already
        // been granted this bridge in onEnable; external needs the operator to
        // paste a link, and the host says so when it is missing.
        val host = SvcBridgeHost(
            relayWorld = world,
            peering = BridgePeering(nodeDir),
            nodeDir = nodeDir,
            speakers = wiring::speaker,
            liveClients = liveClients(config),
            // The membership keys the connection registry indexes, `game:gamertag`.
            // The prefix is what makes each one a key the server can answer; without
            // it every lookup returns no rather than erroring. Plural because a
            // linked Bedrock player is known by both their Java account name and
            // their Xbox gamertag, and registers under whichever their BVC login
            // carried.
            identitiesOf = { id ->
                server.getPlayer(id)
                    ?.let(playerDataProvider::resolveMembershipKeys)
                    ?: emptyList()
            },
            onlinePlayers = { server.onlinePlayers.map { it.uniqueId } },
            onServerThread = { task -> server.scheduler.runTask(this, task) },
            channelFactory = { api ->
                PaperSvcChannelFactory(api, server, playerDataProvider::findByIdentity)
            }
        )
        svcBridgeHost = host

        service.registerPlugin(host.bridge { svcServerPeerlink(config) })
    }

    /**
     * Who already hears this server's audio through a BVC client.
     *
     * Embedded asks over FFI per call; external polls the API and reads a snapshot,
     * because the audio path must not wait on a round trip.
     */
    private fun liveClients(config: ModConfig): LiveClients {
        val embedded = embeddedServer
        if (embedded != null) {
            return LiveClients.direct(embedded::hasLiveClient)
        }

        val handler = HttpRequestHandler(config.bvcServer!!, config.accessToken!!)
        return LiveClients.polled(handler::liveClients)
    }

    /**
     * The BVC server this bridge dials.
     *
     * Embedded reads the server's own link back over FFI once it is running;
     * external takes the operator's configured value.
     */
    // Embedded asks the server it started. External asks the server over `/api/config`,
    // which is unauthenticated and is what removes the hand-copied peerlink.
    private fun svcServerPeerlink(config: ModConfig): String? =
        embeddedServer?.serverPeerlink() ?: pairingEligibility?.resolve()

    // `null` in embedded mode, where the mod grants itself and no code exists to redeem.
    private fun svcPairingRequest(): PairingRequest? {
        if (embeddedServer != null) {
            return null
        }

        return SvcPairing.forExternal(
            pairingEligibility,
            File(dataFolder, "svc-bridge"),
            worlds = { listOfNotNull(relayWorld?.id()) },
            onPaired = { svcBridgeHost?.onPaired() },
        )
    }

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
        relayWorld = RelayWorld(dataFolder)
        // The bridge does not exist until Simple Voice Chat hands over its server API,
        // several steps after this, so the provider reads it through the field rather than
        // holding one. Before then, and on a server without Simple Voice Chat, nobody is
        // bridged.
        playerDataProvider = PaperPlayerDataProvider(
            relayWorld = relayWorld,
            bridgedVoice = { uuid -> svcBridgeHost?.isOnVoice(uuid) ?: false }
        )

        // Native libraries are resolved from the plugin data directory rather than
        // unpacked from the jar. Configured before anything can reach an FFI call,
        // which for the embedded server is its start below.
        val provider = NativeLibraryProvider(
            cacheRoot = dataFolder,
            manifest = NativeManifest.fromResources(),
            fetcher = HttpLibraryFetcher()
        )
        BvcNative.configure(provider)

        // The relay SDK is loaded by bare name by uniffi's generated bindings, so
        // its directory has to be on JNA's search path before the first one runs.
        // The first is BridgePeering, below, while the embedded grant is written.
        if (svcAvailability.isAvailable) {
            try {
                provider.prepareForBareNameLoad("bvc_relay_sdk")
            } catch (e: Exception) {
                logger.severe("Could not resolve the relay SDK library: ${e.message}")
                logger.severe("The Simple Voice Chat bridge will not start.")
                return
            }
        }

        // Initialize embedded server if configured
        var httpHandler: HttpRequestHandler? = null
        if (config.useEmbeddedServer) {
            // Granted before the server starts, because authorization is read from
            // config at startup. Applied only when SVC is present, so a server
            // without it declares no peer it will never see.
            if (svcAvailability.isAvailable) {
                val nodeDir = File(dataFolder, "svc-bridge")
                config.embeddedConfig = (config.embeddedConfig ?: EmbeddedServerConfig()).also {
                    EmbeddedGrant(BridgePeering(nodeDir)).applyTo(it)
                }
            }

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
            httpHandler = HttpRequestHandler(config.bvcServer!!, config.accessToken!!)
            pairingEligibility = PeeringEligibility(httpHandler!!::serverPeerLink)
            positionSender = PositionSender(httpHandler, null)
            audioEventSender = AudioEventSender(httpHandler, null)
            controlSender = ControlSender(httpHandler, null)

            logger.info("Bedrock Voice Chat will connect to: ${config.bvcServer}")
        }

        reportHostCapability(config, httpHandler)
        startSvcBridge(config)

        // Set up audio player manager
        val sender = audioEventSender!!
        audioPlayerManager = PaperAudioPlayerManager(sender, this, relayWorld)

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
            PeerCommand(::svcPairingRequest).addTo(bvc)
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
        // Before the embedded server stops: the pump is parked in nextFrame, and
        // only an explicit shutdown releases it. Left running, plugin disable would
        // hang on a parked call.
        svcBridgeHost?.shutdown()
        svcBridgeHost = null
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
