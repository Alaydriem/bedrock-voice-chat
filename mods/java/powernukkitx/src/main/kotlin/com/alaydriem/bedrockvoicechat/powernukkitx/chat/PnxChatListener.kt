package com.alaydriem.bedrockvoicechat.powernukkitx.chat

import com.alaydriem.bedrockvoicechat.chat.ChatTransport
import org.powernukkitx.Server
import org.powernukkitx.event.EventHandler
import org.powernukkitx.event.EventPriority
import org.powernukkitx.event.Listener
import org.powernukkitx.event.player.PlayerChatEvent
import org.powernukkitx.event.player.PlayerCommandPreprocessEvent
import org.powernukkitx.event.player.PlayerDeathEvent
import org.powernukkitx.event.player.PlayerJoinEvent
import org.powernukkitx.event.player.PlayerQuitEvent
import org.powernukkitx.event.server.RemoteServerCommandEvent
import org.powernukkitx.event.server.ServerCommandEvent
import org.powernukkitx.lang.TextContainer
import org.powernukkitx.utils.TextFormat

/**
 * Bridges PowerNukkitX chat and the BVC chat channel.
 *
 * Player chat is one event; deaths, joins, quits and `/say` are each another. Death, join
 * and quit messages arrive as translation keys, which [translator] renders as text.
 *
 * PowerNukkitX fires no event for a server broadcast, so a line broadcast by another
 * plugin is not relayed, and a line from the app is never echoed back.
 */
class PnxChatListener(
    private val channel: ChatTransport,
    private val translator: PnxMessageTranslator,
    private val server: Server,
) : Listener {
    companion object {
        private val SAY = Regex("^/?(?:minecraft:)?say\\s+(.+)$", RegexOption.IGNORE_CASE)

        // The permission PowerNukkitX's own SayCommand checks.
        private const val SAY_PERMISSION = "nukkit.command.say"

        /**
         * The argument of a `/say`, or null when the command is anything else.
         *
         * The namespaced alias counts, and a command that merely begins with those letters
         * does not.
         */
        @JvmStatic
        fun sayArgument(command: String): String? =
            SAY.matchEntire(command.trim())?.groupValues?.get(1)?.trim()?.ifEmpty { null }

        /**
         * Text safe to show as one chat line: colour codes removed and line breaks flattened.
         */
        @JvmStatic
        fun sanitize(text: String): String =
            TextFormat.clean(text).replace('\r', ' ').replace('\n', ' ')
    }

    /**
     * A line a player typed.
     *
     * MONITOR priority and cancellation-aware: a message another plugin blocked was never seen
     * in game, and relaying it would show the app what the server suppressed.
     */
    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    fun onChat(event: PlayerChatEvent) {
        channel.report(event.getPlayer().getName(), TextFormat.clean(event.getMessage()))
    }

    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    fun onDeath(event: PlayerDeathEvent) {
        relayServerMessage(event.getDeathMessage())
    }

    @EventHandler(priority = EventPriority.MONITOR)
    fun onJoin(event: PlayerJoinEvent) {
        relayServerMessage(event.getJoinMessage())
    }

    @EventHandler(priority = EventPriority.MONITOR)
    fun onQuit(event: PlayerQuitEvent) {
        relayServerMessage(event.getQuitMessage())
    }

    /**
     * `/say` from the console.
     *
     * This fires before the command runs, so one that then fails is still relayed.
     */
    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    fun onConsoleCommand(event: ServerCommandEvent) {
        relayConsoleSay(event.getCommand())
    }

    /**
     * `/say` over RCON.
     *
     * A separate handler because the event declares its own handler list, so it never reaches
     * the one above despite being a subclass.
     */
    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    fun onRemoteCommand(event: RemoteServerCommandEvent) {
        relayConsoleSay(event.getCommand())
    }

    /** `/say` from a player. */
    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    fun onPlayerCommand(event: PlayerCommandPreprocessEvent) {
        val text = sayArgument(event.getMessage()) ?: return
        // Without the check, a player who is about to be refused would still be relayed.
        if (!event.getPlayer().hasPermission(SAY_PERMISSION)) {
            return
        }
        channel.event("[${event.getPlayer().getName()}] $text")
    }

    /**
     * Broadcasts a line composed in the app, formatted to match vanilla chat.
     *
     * A broadcast fires no chat event, so the line is never reported back as chat.
     */
    fun say(author: String, text: String) {
        server.broadcastMessage("<${sanitize(author)}> ${sanitize(text)}")
    }

    // Null or empty when a plugin has suppressed the message, which means nobody saw it.
    private fun relayServerMessage(message: TextContainer?) {
        if (message == null || message.text.isNullOrEmpty()) {
            return
        }
        val text = translator.render(message)
        if (text.isNotBlank()) {
            channel.event(text)
        }
    }

    private fun relayConsoleSay(command: String) {
        val text = sayArgument(command) ?: return
        channel.event("[Server] $text")
    }
}
