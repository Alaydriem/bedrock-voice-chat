package com.alaydriem.bedrockvoicechat.fabric.chat

import com.alaydriem.bedrockvoicechat.chat.ChatChannel
import net.minecraft.network.chat.Component
import net.minecraft.server.MinecraftServer

/**
 * Bridges Fabric's chat and the BVC chat channel.
 */
class FabricChatListener(
    private val channel: ChatChannel,
    private val server: MinecraftServer
) {
    /** Reports a line a player typed. */
    fun onChat(playerName: String, message: String) {
        channel.report(playerName, message)
    }

    /**
     * Broadcasts a line composed in the app, formatted to match vanilla so it is
     * indistinguishable from something typed in game.
     *
     * Queued onto the server thread: the socket delivers on its own, and the player manager is
     * not safe to touch from there.
     */
    fun say(author: String, text: String) {
        server.execute {
            server.playerList.broadcastSystemMessage(
                Component.literal("<$author> $text"),
                false
            )
        }
    }
}
