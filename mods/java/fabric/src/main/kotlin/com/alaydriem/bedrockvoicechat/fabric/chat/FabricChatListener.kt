package com.alaydriem.bedrockvoicechat.fabric.chat

import com.alaydriem.bedrockvoicechat.chat.ChatTransport
import net.minecraft.network.chat.Component
import net.minecraft.server.MinecraftServer

/**
 * Bridges Fabric's chat and the BVC chat channel.
 *
 * Two sources rather than one. Player chat arrives on `CHAT_MESSAGE`; deaths, joins, leaves
 * and `/say` all reach players as *system* messages, so they arrive on `GAME_MESSAGE`. The
 * no-net Bedrock path sees every one of them free, because the proxy reads whatever the realm
 * sends — a mod has to subscribe to both, and only subscribing to the first is why deaths and
 * joins showed on Bedrock and nowhere else.
 */
class FabricChatListener(
    private val channel: ChatTransport,
    private val server: MinecraftServer
) {
    /** A line a player typed. */
    fun onChat(playerName: String, message: String) {
        channel.report(playerName, message)
    }

    /** Something the server said: a death, a join, a leave, a broadcast. */
    fun onGameMessage(message: Component) {
        val text = message.string
        if (isEcho(text)) {
            return
        }
        channel.event(text)
    }

    /**
     * Broadcasts a line composed in the app, formatted to match vanilla so it is
     * indistinguishable from something typed in game.
     *
     * Queued onto the server thread: the transport delivers on its own, and the player list is
     * not safe to touch from there.
     *
     * This goes out as a system message, so [onGameMessage] would relay it straight back — and
     * the server already fanned this line out to clients when it accepted the send, so the app
     * would show it twice. Suppressed by remembering the line for one message.
     */
    fun say(author: String, text: String) {
        val line = "<$author> $text"
        suppressed = line
        server.execute {
            server.playerList.broadcastSystemMessage(Component.literal(line), false)
        }
    }

    /** The one app-sent line currently in flight, so its own broadcast is not relayed back. */
    @Volatile
    private var suppressed: String? = null

    private fun isEcho(text: String): Boolean {
        if (suppressed == text) {
            suppressed = null
            return true
        }
        return false
    }
}
