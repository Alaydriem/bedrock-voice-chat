package com.alaydriem.bedrockvoicechat.paper.chat

import com.alaydriem.bedrockvoicechat.chat.ChatChannel
import io.papermc.paper.event.player.AsyncChatEvent
import net.kyori.adventure.text.Component
import net.kyori.adventure.text.serializer.plain.PlainTextComponentSerializer
import org.bukkit.Bukkit
import org.bukkit.event.EventHandler
import org.bukkit.event.EventPriority
import org.bukkit.event.Listener

/**
 * Bridges Paper's chat and the BVC chat channel.
 */
class PaperChatListener(private val channel: ChatChannel) : Listener {
    companion object {
        private val PLAIN = PlainTextComponentSerializer.plainText()
    }

    /**
     * Reports a line a player typed.
     *
     * MONITOR priority and cancellation-aware: a message another plugin blocked was never seen
     * in game, and relaying it would let the app show what the server suppressed.
     */
    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    fun onChat(event: AsyncChatEvent) {
        channel.report(event.player.name, PLAIN.serialize(event.message()))
    }

    /**
     * Broadcasts a line composed in the app, formatted to match vanilla so it is
     * indistinguishable from something typed in game.
     *
     * `Bukkit.broadcast` does not fire `AsyncChatEvent`, so this is never reported back and
     * there is nothing to suppress.
     */
    fun say(author: String, text: String) {
        Bukkit.broadcast(Component.text("<$author> $text"))
    }
}
