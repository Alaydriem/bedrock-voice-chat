package com.alaydriem.bedrockvoicechat.fabric

import com.alaydriem.bedrockvoicechat.audio.AudioEventSender
import com.alaydriem.bedrockvoicechat.chat.AsyncChatTransport
import com.alaydriem.bedrockvoicechat.chat.ChatChannel
import com.alaydriem.bedrockvoicechat.chat.ChatTransport
import com.alaydriem.bedrockvoicechat.chat.FfiChatTransport
import com.alaydriem.bedrockvoicechat.control.ControlSender
import com.alaydriem.bedrockvoicechat.fabric.chat.FabricChatListener
import net.fabricmc.fabric.api.message.v1.ServerMessageEvents
import com.alaydriem.bedrockvoicechat.dto.Dimension
import com.alaydriem.bedrockvoicechat.dto.Payload
import com.alaydriem.bedrockvoicechat.dto.PlayerData
import com.alaydriem.bedrockvoicechat.fabric.audio.FabricAudioPlayerManager
import com.alaydriem.bedrockvoicechat.fabric.audio.JukeboxListener
import com.alaydriem.bedrockvoicechat.fabric.commands.ControlCommands
import com.alaydriem.bedrockvoicechat.fabric.commands.DiscCommand
import com.alaydriem.bedrockvoicechat.config.ModConfig
import com.alaydriem.bedrockvoicechat.config.generated.EmbeddedServerConfig
import com.alaydriem.bedrockvoicechat.native.BvcNative
import com.alaydriem.bedrockvoicechat.native.HostCapabilityCheck
import com.alaydriem.bedrockvoicechat.native.HostCapabilitySender
import com.alaydriem.bedrockvoicechat.native.HttpLibraryFetcher
import com.alaydriem.bedrockvoicechat.native.NativeLibraryProvider
import com.alaydriem.bedrockvoicechat.native.NativeManifest
import com.alaydriem.bedrockvoicechat.fabric.svc.FabricSvcChannelFactory
import com.alaydriem.bedrockvoicechat.fabric.svc.FabricSvcPlugin
import com.alaydriem.bedrockvoicechat.fabric.svc.FabricSvcWiring
import com.alaydriem.bedrockvoicechat.native.PositionSender
import com.alaydriem.bedrockvoicechat.svc.BridgePeering
import com.alaydriem.bedrockvoicechat.svc.EmbeddedGrant
import com.alaydriem.bedrockvoicechat.svc.LiveClients
import com.alaydriem.bedrockvoicechat.svc.RelayWorld
import com.alaydriem.bedrockvoicechat.svc.SvcAvailability
import com.alaydriem.bedrockvoicechat.svc.SvcBridgeHost
import net.minecraft.server.MinecraftServer
import com.alaydriem.bedrockvoicechat.network.HttpRequestHandler
import com.alaydriem.bedrockvoicechat.server.BvcServerManager
import net.fabricmc.api.ModInitializer
import net.fabricmc.loader.api.FabricLoader
import net.fabricmc.fabric.api.entity.event.v1.ServerLivingEntityEvents
import net.fabricmc.fabric.api.entity.event.v1.ServerPlayerEvents
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerLifecycleEvents
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerTickEvents
import net.fabricmc.fabric.api.networking.v1.ServerPlayConnectionEvents
import net.minecraft.server.level.ServerPlayer
import net.minecraft.server.level.ServerLevel
import org.slf4j.LoggerFactory

class FabricMod : ModInitializer {
    private val logger = LoggerFactory.getLogger("Bedrock Voice Chat")

    private val configProvider = FabricConfigProvider()
    private lateinit var playerDataProvider: FabricPlayerDataProvider

    private var embeddedServer: BvcServerManager? = null
    private var relayWorld: RelayWorld? = null
    private var svcBridgeHost: SvcBridgeHost? = null

    // One instance: it caches its answer and logs on first resolve, so a second
    // instance reports the same detection a second time.
    private val svcAvailability = SvcAvailability()
    private var chatChannel: ChatTransport? = null
    private var chatSocket: ChatChannel? = null
    private var positionSender: PositionSender? = null
    private var audioPlayerManager: FabricAudioPlayerManager? = null
    private var tickCounter = 0
    private var minimumPlayers = 1

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
                    cacheRoot = FabricLoader.getInstance().configDir.resolve("bedrock-voice-chat").toFile(),
                    manifest = manifest,
                    fetcher = HttpLibraryFetcher()
                )
                val report = HostCapabilityCheck(
                    provider, manifest, HttpLibraryFetcher(), manifest.release, true
                ).run()
                report?.let { HostCapabilitySender(httpHandler, embedded).send(it) }
            } catch (e: Exception) {
                logger.debug("Host capability check did not complete: {}", e.message)
            }
        }, "bvc-host-capability").apply { isDaemon = true }.start()
    }

    /**
     * Builds the bridge and hands it to the entrypoint SVC will call.
     *
     * Runs at server start rather than mod init, because the channel factory and the
     * speaker lookup both need levels and a player list that do not exist earlier.
     *
     * The availability check runs before any bridge class is touched, so a server
     * without SVC never loads one — those classes name SVC types and would fail to
     * link.
     */
    private fun startSvcBridge(server: MinecraftServer, config: ModConfig) {
        if (!svcAvailability.isAvailable) {
            return
        }

        val world = relayWorld ?: return
        val provider = playerDataProvider
        val nodeDir = FabricLoader.getInstance().configDir
            .resolve("bedrock-voice-chat")
            .resolve("svc-bridge")
            .toFile()
        val wiring = FabricSvcWiring(server, provider)

        val host = SvcBridgeHost(
            relayWorld = world,
            peering = BridgePeering(nodeDir),
            nodeDir = nodeDir,
            speakers = wiring::speaker,
            liveClients = liveClients(config),
            // The membership key the connection registry indexes, which is the same
            // canonical name the position feed sends — a Bedrock player on a Geyser
            // server by gamertag, not by their prefixed Java username.
            identityOf = { id ->
                server.playerList.getPlayer(id)?.let(provider::resolveCanonicalName)
            },
            channelFactory = { api -> FabricSvcChannelFactory(api, server) }
        )
        svcBridgeHost = host

        FabricSvcPlugin.attach(
            host.bridge {
                embeddedServer?.serverPeerlink() ?: config.svcBridgePeerlink
            }
        )
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

    override fun onInitialize() {
        logger.info("Initializing Bedrock Voice Chat")

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
        // Beside the existing world_uuid_*.txt files, so everything this server
        // mints for BVC lives in one directory.
        relayWorld = RelayWorld(
            FabricLoader.getInstance().configDir.resolve("bedrock-voice-chat").toFile()
        )
        // The bridge does not exist until the server starts, several steps after this, so
        // the provider reads it through the field rather than holding one. Before then, and
        // on a server without Simple Voice Chat, nobody is bridged.
        playerDataProvider = FabricPlayerDataProvider(
            relayWorld = relayWorld,
            bridgedVoice = { uuid -> svcBridgeHost?.isOnVoice(uuid) ?: false }
        )

        // Native libraries are resolved from the mod's config directory rather than
        // unpacked from the jar. Configured before anything can reach an FFI call,
        // which for the embedded server is its start below.
        val provider = NativeLibraryProvider(
            cacheRoot = FabricLoader.getInstance().configDir.resolve("bedrock-voice-chat").toFile(),
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
                logger.error("Could not resolve the relay SDK library: {}", e.message)
                logger.error("The Simple Voice Chat bridge will not start.")
                return
            }
        }

        var controlSender: ControlSender? = null
        var httpHandler: HttpRequestHandler? = null

        if (config.useEmbeddedServer) {
            // Granted before the server starts, because authorization is read from
            // config at startup. Applied only when SVC is present, so a server
            // without it declares no peer it will never see.
            if (svcAvailability.isAvailable) {
                val nodeDir = FabricLoader.getInstance().configDir
                    .resolve("bedrock-voice-chat")
                    .resolve("svc-bridge")
                    .toFile()
                config.embeddedConfig = (config.embeddedConfig ?: EmbeddedServerConfig()).also {
                    EmbeddedGrant(BridgePeering(nodeDir)).applyTo(it)
                }
            }

            embeddedServer = BvcServerManager(config, configProvider)
            if (!embeddedServer!!.start()) {
                logger.error("Failed to start embedded server - falling back to disabled state")
                embeddedServer = null
                return
            }

            positionSender = PositionSender(null, embeddedServer)
            val audioEventSender = AudioEventSender(null, embeddedServer)
            audioPlayerManager = FabricAudioPlayerManager(audioEventSender, relayWorld)
            controlSender = ControlSender(null, embeddedServer)

            val quicPort = embeddedServer?.effectiveConfig()?.server?.quicPort
            logger.info("Bedrock Voice Chat using embedded server (QUIC port: {})", quicPort)
        } else {
            httpHandler = HttpRequestHandler(config.bvcServer!!, config.accessToken!!)
            positionSender = PositionSender(httpHandler, null)
            val audioEventSender = AudioEventSender(httpHandler, null)
            audioPlayerManager = FabricAudioPlayerManager(audioEventSender, relayWorld)
            controlSender = ControlSender(httpHandler, null)

            logger.info("Bedrock Voice Chat will connect to: {}", config.bvcServer)
        }

        reportHostCapability(config, httpHandler)

        JukeboxListener(audioPlayerManager!!, playerDataProvider::getWorldUuid).register()
        DiscCommand.register()
        ControlCommands.register(controlSender, playerDataProvider::resolveCanonicalName)

        ServerPlayConnectionEvents.JOIN.register { handler, _, _ ->
            playerDataProvider.addPlayer(handler.player)
        }

        ServerPlayConnectionEvents.DISCONNECT.register { handler, _ ->
            val player = handler.player
            val sender = positionSender
            val canonicalName = playerDataProvider.resolveCanonicalName(player)

            // Capture phantom data before removePlayer() clears state
            val phantom = if (sender != null) {
                val worldUuid = playerDataProvider.getWorldUuid(player.level() as ServerLevel)
                PlayerData.disconnected(
                    name = canonicalName,
                    dimension = Dimension.Minecraft.DEATH,
                    worldUuid = worldUuid,
                    playerUuid = player.getUUID().toString()
                )
            } else null

            playerDataProvider.removePlayer(player)

            if (phantom != null && sender != null) {
                val payload = Payload(playerDataProvider.getGameType(), listOf(phantom))
                sender.send(payload)
                logger.info("Sent disconnect phantom for player: {}", canonicalName)
            }
        }

        ServerLivingEntityEvents.AFTER_DEATH.register { entity, _ ->
            if (entity is ServerPlayer) {
                playerDataProvider.markDead(entity)
            }
        }

        ServerPlayerEvents.AFTER_RESPAWN.register { _, newPlayer, _ ->
            playerDataProvider.markAlive(newPlayer)
        }

        ServerTickEvents.END_SERVER_TICK.register { server ->
            playerDataProvider.server = server

            tickCounter++
            if (tickCounter >= 5) {
                tickCounter = 0
                tick()
            }
        }

        // Levels do not exist until the server has started, and chat spans all of them:
        // Fabric mints a world id per dimension while chat is server-wide.
        ServerLifecycleEvents.SERVER_STARTED.register { server ->
            startChatChannel(server, config)
            startSvcBridge(server, config)
        }

        ServerLifecycleEvents.SERVER_STOPPING.register { _ ->
            chatChannel?.stop()
            chatChannel = null
            audioPlayerManager?.shutdown()
            // Before the embedded server stops: the pump is parked in nextFrame,
            // and only an explicit shutdown releases it.
            svcBridgeHost?.shutdown()
            svcBridgeHost = null
            FabricSvcPlugin.detach()
            embeddedServer?.stop()
        }
    }

    /**
     * Opens the chat relay to the BVC server.
     *
     * Every dimension's world id is declared, because a line typed in the overworld has to
     * reach somebody standing in the nether. The primary level supplies the canonical id and
     * the name the app's picker shows.
     */
    private fun startChatChannel(
        server: net.minecraft.server.MinecraftServer,
        config: com.alaydriem.bedrockvoicechat.config.ModConfig
    ) {
        val worlds = server.allLevels.map { playerDataProvider.getWorldUuid(it) }
        if (worlds.isEmpty()) {
            logger.warn("Bedrock Voice Chat chat relay not started (no levels loaded)")
            return
        }
        val worldName = server.motd ?: "Minecraft server"

        var listener: FabricChatListener? = null
        val onSay: (String, String) -> Unit = { author, text -> listener?.say(author, text) }

        // Chosen by mode, exactly as ControlSender and AudioEventSender already are. Embedded
        // shares this process, so it calls into the server rather than dialling a socket back
        // into its own address space.
        val base: ChatTransport = if (config.useEmbeddedServer) {
            val embedded = embeddedServer
            if (embedded == null) {
                logger.warn("Bedrock Voice Chat chat relay not started (embedded server not running)")
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

        // CHAT_MESSAGE and GAME_MESSAGE both fire on the main server thread, and neither
        // transport is safe to block it with.
        val transport = AsyncChatTransport(base)

        listener = FabricChatListener(transport, server)

        ServerMessageEvents.CHAT_MESSAGE.register { message, sender, _ ->
            listener.onChat(sender.name.string, message.signedContent())
        }

        // Deaths, joins and leaves reach players as system messages rather than as chat, so
        // they arrive here and not on CHAT_MESSAGE.
        ServerMessageEvents.GAME_MESSAGE.register { _, message, _ ->
            listener.onGameMessage(message)
        }

        // /say and /me are neither chat nor a system message: command output has its own
        // event. The bound chat type is what renders "[Server] hello" rather than the bare
        // argument.
        ServerMessageEvents.COMMAND_MESSAGE.register { message, _, params ->
            listener.onGameMessage(params.decorate(message.decoratedContent()))
        }

        chatChannel = transport
        transport.start()
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
