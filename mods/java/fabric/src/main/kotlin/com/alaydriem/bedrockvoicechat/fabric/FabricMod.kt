package com.alaydriem.bedrockvoicechat.fabric

import com.alaydriem.bedrockvoicechat.audio.AudioEventSender
import com.alaydriem.bedrockvoicechat.chat.ChatChannel
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
import com.alaydriem.bedrockvoicechat.native.PositionSender
import com.alaydriem.bedrockvoicechat.network.HttpRequestHandler
import com.alaydriem.bedrockvoicechat.server.BvcServerManager
import net.fabricmc.api.ModInitializer
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
    private var chatChannel: ChatChannel? = null
    private var positionSender: PositionSender? = null
    private var audioPlayerManager: FabricAudioPlayerManager? = null
    private var tickCounter = 0
    private var minimumPlayers = 1

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
        playerDataProvider = FabricPlayerDataProvider()

        var controlSender: ControlSender? = null

        if (config.useEmbeddedServer) {
            embeddedServer = BvcServerManager(config, configProvider)
            if (!embeddedServer!!.start()) {
                logger.error("Failed to start embedded server - falling back to disabled state")
                embeddedServer = null
                return
            }

            positionSender = PositionSender(null, embeddedServer)
            val audioEventSender = AudioEventSender(null, embeddedServer)
            audioPlayerManager = FabricAudioPlayerManager(audioEventSender)
            controlSender = ControlSender(null, embeddedServer)

            val embedded = config.embeddedConfig
            logger.info("Bedrock Voice Chat using embedded server (QUIC port: {})", embedded?.quicPort ?: 8443)
        } else {
            val httpHandler = HttpRequestHandler(config.bvcServer!!, config.accessToken!!)
            positionSender = PositionSender(httpHandler, null)
            val audioEventSender = AudioEventSender(httpHandler, null)
            audioPlayerManager = FabricAudioPlayerManager(audioEventSender)
            controlSender = ControlSender(httpHandler, null)

            logger.info("Bedrock Voice Chat will connect to: {}", config.bvcServer)
        }

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
        }

        ServerLifecycleEvents.SERVER_STOPPING.register { _ ->
            chatChannel?.close()
            chatChannel = null
            audioPlayerManager?.shutdown()
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
        val serverUrl = config.bvcServer
        val token = config.accessToken
        if (serverUrl == null || token == null) {
            // Embedded mode has no external URL to dial.
            logger.info("Bedrock Voice Chat chat relay not started (no external server configured)")
            return
        }

        val worlds = server.allLevels.map { playerDataProvider.getWorldUuid(it) }
        if (worlds.isEmpty()) {
            logger.warn("Bedrock Voice Chat chat relay not started (no levels loaded)")
            return
        }

        var listener: FabricChatListener? = null
        val channel = ChatChannel(
            serverUrl = serverUrl,
            accessToken = token,
            worldUuid = worlds.first(),
            worldName = server.motd ?: "Minecraft server",
            worlds = worlds,
            onSay = { author, text -> listener?.say(author, text) },
            send = { body -> chatChannel?.sendOverSocket(body) }
        )
        listener = FabricChatListener(channel, server)

        ServerMessageEvents.CHAT_MESSAGE.register { message, sender, _ ->
            listener.onChat(sender.name.string, message.signedContent())
        }

        chatChannel = channel
        channel.connect()
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
