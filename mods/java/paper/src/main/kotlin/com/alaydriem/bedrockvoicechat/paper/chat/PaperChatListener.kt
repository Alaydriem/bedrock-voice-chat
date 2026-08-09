package com.alaydriem.bedrockvoicechat.paper.chat

import com.alaydriem.bedrockvoicechat.chat.ChatTransport
import io.papermc.paper.event.player.AsyncChatEvent
import net.kyori.adventure.text.Component
import net.kyori.adventure.text.serializer.plain.PlainTextComponentSerializer
import org.bukkit.Bukkit
import org.bukkit.event.EventHandler
import org.bukkit.event.EventPriority
import org.bukkit.event.Listener
import org.bukkit.event.entity.PlayerDeathEvent
import org.bukkit.event.player.PlayerCommandPreprocessEvent
import org.bukkit.event.player.PlayerJoinEvent
import org.bukkit.event.player.PlayerQuitEvent
import org.bukkit.event.server.BroadcastMessageEvent
import org.bukkit.event.server.RemoteServerCommandEvent
import org.bukkit.event.server.ServerCommandEvent

/**
 * Bridges Paper's chat and the BVC chat channel.
 *
 * Player chat is one event; everything else a player reads in the chat box is a different one.
 * The no-net Bedrock path gets all of it free because the proxy sees every message the realm
 * sends — a mod has to subscribe to each kind, and missing one is why deaths and joins showed
 * on Bedrock and nowhere else.
 */
class PaperChatListener(private val channel: ChatTransport) : Listener {
    companion object {
        private val PLAIN = PlainTextComponentSerializer.plainText()

        private val SAY = Regex("^/?(?:minecraft:)?say\\s+(.+)$", RegexOption.IGNORE_CASE)

        /**
         * The argument of a `/say`, or null when the command is anything else.
         *
         * The namespaced alias counts, and a command that merely begins with those letters
         * does not.
         */
        @JvmStatic
        fun sayArgument(command: String): String? =
            SAY.matchEntire(command.trim())?.groupValues?.get(1)?.trim()?.ifEmpty { null }
    }

    /**
     * A line a player typed.
     *
     * MONITOR priority and cancellation-aware: a message another plugin blocked was never seen
     * in game, and relaying it would show the app what the server suppressed.
     */
    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    fun onChat(event: AsyncChatEvent) {
        channel.report(event.player.name, PLAIN.serialize(event.message()))
    }

    /** A death message, as the server worded it. */
    @EventHandler(priority = EventPriority.MONITOR)
    fun onDeath(event: PlayerDeathEvent) {
        // Null when a plugin has suppressed the message, which means nobody saw it in game.
        val message = event.deathMessage() ?: return
        channel.event(PLAIN.serialize(message))
    }

    @EventHandler(priority = EventPriority.MONITOR)
    fun onJoin(event: PlayerJoinEvent) {
        val message = event.joinMessage() ?: return
        channel.event(PLAIN.serialize(message))
    }

    @EventHandler(priority = EventPriority.MONITOR)
    fun onQuit(event: PlayerQuitEvent) {
        val message = event.quitMessage() ?: return
        channel.event(PLAIN.serialize(message))
    }

    /**
     * `/say`, `/me`, and anything a plugin broadcasts.
     *
     * Cancellation-aware for the same reason as chat.
     */
    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    fun onBroadcast(event: BroadcastMessageEvent) {
        val text = PLAIN.serialize(event.message())
        if (isEcho(text)) {
            return
        }
        channel.event(text)
    }

    /**
     * `/say` from the console.
     *
     * Paper exposes no event for a vanilla system message. `BroadcastMessageEvent` carries
     * plugin broadcasts only, which is why deaths and joins arrive and `/say` did not. The
     * command is therefore read where it is issued, and worded the way vanilla words it.
     *
     * This fires before the command runs, so one that then fails is still relayed. That is the
     * cost of the missing event, and it is the smaller error of the two.
     */
    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    fun onConsoleCommand(event: ServerCommandEvent) {
        relayConsoleSay(event.command)
    }

    /**
     * `/say` over RCON, which is how most hosting panels send console commands.
     *
     * A separate handler because the event declares its own handler list, so it never reaches
     * the one above despite being a subclass.
     */
    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    fun onRemoteCommand(event: RemoteServerCommandEvent) {
        relayConsoleSay(event.command)
    }

    private fun relayConsoleSay(command: String) {
        val text = sayArgument(command) ?: return
        channel.event("[Server] $text")
    }

    /** `/say` from a player. */
    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    fun onPlayerCommand(event: PlayerCommandPreprocessEvent) {
        val text = sayArgument(event.message) ?: return
        // Vanilla commands are permissioned under this name. Without the check, a player who
        // is about to be refused would still be relayed.
        if (!event.player.hasPermission("minecraft.command.say")) {
            return
        }
        channel.event("[${event.player.name}] $text")
    }

    /**
     * Broadcasts a line composed in the app, formatted to match vanilla so it is
     * indistinguishable from something typed in game.
     *
     * `Bukkit.broadcast` does not fire `AsyncChatEvent`, so this is never reported back as
     * chat. It *does* fire `BroadcastMessageEvent`, which [onBroadcast] would relay — the
     * server already fans this line out to clients when it accepts the send, so relaying the
     * echo would show it twice. Suppressed by remembering the line for one broadcast.
     */
    fun say(author: String, text: String) {
        val line = "<$author> $text"
        suppressed = line
        Bukkit.broadcast(Component.text(line))
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
