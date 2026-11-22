package com.alaydriem.bedrockvoicechat.powernukkitx.commands

import com.alaydriem.bedrockvoicechat.admin.AdminAction
import com.alaydriem.bedrockvoicechat.admin.AdminConsole
import com.alaydriem.bedrockvoicechat.control.ControlAction
import com.alaydriem.bedrockvoicechat.control.ControlSender
import com.alaydriem.bedrockvoicechat.control.JoinReply
import com.alaydriem.bedrockvoicechat.powernukkitx.audio.PnxDisc
import org.cloudburstmc.protocol.bedrock.data.command.CommandParamType
import org.powernukkitx.Player
import org.powernukkitx.command.Command
import org.powernukkitx.command.CommandSender
import org.powernukkitx.command.ConsoleCommandSender
import org.powernukkitx.command.RemoteConsoleCommandSender
import org.powernukkitx.command.data.CommandEnum
import org.powernukkitx.command.data.CommandParameter
import org.powernukkitx.command.tree.ParamList
import org.powernukkitx.command.tree.node.PlayersNode
import org.powernukkitx.command.tree.node.StringNode
import org.powernukkitx.command.utils.CommandLogger
import org.powernukkitx.item.Item

/**
 * `/bvc`: disc, give, mute, deafen, record, volume, hear, jukebox, group and admin.
 *
 * Each subcommand is one overload of the parameter tree, keyed by the names below, and
 * hands its work to the same `common` handler the Paper command uses.
 *
 * The actor id is the player's canonical name, the identity the position feed registers.
 * A typed target resolves to an online player and passes through the same mapping, so
 * per-player preferences key on the same gamertag everywhere downstream.
 *
 * [onServerThread] carries a control result back to the main thread: the external server
 * answers on the HTTP client's executor, and sending a player a message fires plugin events.
 */
class BvcCommand(
    private val controlSender: ControlSender,
    private val adminConsole: AdminConsole,
    private val canonicalName: (Player) -> String,
    private val onlinePlayers: () -> Collection<Player>,
    private val onServerThread: (Runnable) -> Unit,
    private val createDisc: (String) -> Item = PnxDisc::create,
) : Command("bvc", "Bedrock Voice Chat commands") {

    init {
        commandParameters.clear()
        commandParameters[DISC] = arrayOf(
            literal("disc"),
            CommandParameter.newType("audio_id", CommandParamType.MESSAGE),
        )
        commandParameters[GIVE] = arrayOf(
            literal("give"),
            CommandParameter.newType("target", CommandParamType.SELECTION, PlayersNode()),
            CommandParameter.newType("audio_id", CommandParamType.MESSAGE),
        )
        commandParameters[MUTE] = arrayOf(literal("mute"), toggle())
        commandParameters[DEAFEN] = arrayOf(literal("deafen"), toggle())
        commandParameters[RECORD] = arrayOf(literal("record"), toggle())
        commandParameters[VOLUME] = arrayOf(literal("volume"), playerName(), level())
        commandParameters[HEAR] = arrayOf(literal("hear"), playerName(), toggle())
        commandParameters[JUKEBOX_MUTE] = arrayOf(literal("jukebox"), literal("mute"), toggle())
        commandParameters[JUKEBOX_VOLUME] = arrayOf(literal("jukebox"), literal("volume"), level())
        commandParameters[GROUP_CREATE] = arrayOf(literal("group"), literal("create"))
        commandParameters[GROUP_JOIN] = arrayOf(
            literal("group"),
            literal("join"),
            CommandParameter.newType("code", CommandParamType.RAW_TEXT, StringNode()),
        )
        commandParameters[GROUP_LEAVE] = arrayOf(literal("group"), literal("leave"))
        commandParameters[ADMIN] = arrayOf(
            literal("admin"),
            CommandParameter.newEnum("action", AdminAction.entries.map { it.value }.toTypedArray()),
            CommandParameter.newType("gamertag", CommandParamType.RAW_TEXT, StringNode()),
        )
        enableParamTree()
    }

    override fun execute(
        sender: CommandSender,
        commandLabel: String,
        result: Map.Entry<String, ParamList>,
        log: CommandLogger,
    ): Int {
        val params = result.value
        return when (result.key) {
            DISC -> giveOwnDisc(sender, params.getResult(1))
            GIVE -> giveDiscs(sender, params.getResult(1), params.getResult(2))
            MUTE -> dispatch(sender, ControlAction.Mute(params.getResult(1)))
            DEAFEN -> dispatch(sender, ControlAction.Deafen(params.getResult(1)))
            RECORD -> dispatch(sender, ControlAction.Record(params.getResult(1)))
            VOLUME -> {
                val level: Int = params.getResult(2)
                if (!checkLevel(sender, level)) {
                    0
                } else {
                    dispatchTargeted(sender, params.getResult(1)) { target -> ControlAction.Volume(target, level) }
                }
            }
            HEAR -> {
                val on: Boolean = params.getResult(2)
                dispatchTargeted(sender, params.getResult(1)) { target -> ControlAction.Hear(target, on) }
            }
            // The plane carries "heard", the command reads as "mute".
            JUKEBOX_MUTE -> dispatch(
                sender,
                ControlAction.Hear(ControlAction.JUKEBOX_TARGET, !params.getResult<Boolean>(2)),
            )
            JUKEBOX_VOLUME -> {
                val level: Int = params.getResult(2)
                if (!checkLevel(sender, level)) {
                    0
                } else {
                    dispatch(sender, ControlAction.Volume(ControlAction.JUKEBOX_TARGET, level))
                }
            }
            GROUP_CREATE -> createGroup(sender)
            GROUP_JOIN -> dispatch(sender, ControlAction.JoinGroup(params.getResult(2)))
            GROUP_LEAVE -> dispatch(sender, ControlAction.LeaveGroup)
            ADMIN -> admin(sender, params.getResult(1), params.getResult(2))
            else -> 0
        }
    }

    private fun giveOwnDisc(sender: CommandSender, audioId: String): Int {
        if (!sender.hasPermission(DISC_PERMISSION)) {
            sender.sendMessage("You do not have permission to use /bvc disc")
            return 0
        }
        if (sender !is Player) {
            sender.sendMessage(
                "/bvc disc must be run by a player. Use /bvc give <player> <audio_id> from the console."
            )
            return 0
        }
        giveDisc(sender, audioId)
        sender.sendMessage("Gave BVC audio disc: $audioId")
        return 1
    }

    private fun giveDiscs(sender: CommandSender, targets: List<Player?>, audioId: String): Int {
        if (!sender.hasPermission(GIVE_PERMISSION)) {
            sender.sendMessage("You do not have permission to use /bvc give")
            return 0
        }
        val players = targets.filterNotNull()
        if (players.isEmpty()) {
            sender.sendMessage("No matching player found")
            return 0
        }
        for (player in players) {
            giveDisc(player, audioId)
        }
        sender.sendMessage("Gave BVC audio disc '$audioId' to ${players.size} player(s)")
        return 1
    }

    // A full inventory drops the disc at the player's feet rather than losing it.
    private fun giveDisc(player: Player, audioId: String) {
        val leftovers = player.getInventory().addItem(createDisc(audioId))
        for (item in leftovers) {
            if (!item.isNull()) {
                player.dropItem(item)
            }
        }
    }

    private fun checkLevel(sender: CommandSender, level: Int): Boolean {
        if (level in 0..ControlAction.MAX_LEVEL) {
            return true
        }
        sender.sendMessage("[BVC] Level must be between 0 and ${ControlAction.MAX_LEVEL}")
        return false
    }

    private fun dispatchTargeted(sender: CommandSender, typed: String, build: (String) -> ControlAction): Int {
        val resolved = onlinePlayers().firstOrNull { it.getName().equals(typed, ignoreCase = true) }
        if (resolved == null) {
            sender.sendMessage("[BVC] Unknown player: $typed")
            return 0
        }
        return dispatch(sender, build(canonicalName(resolved)))
    }

    private fun dispatch(sender: CommandSender, action: ControlAction): Int {
        val player = requirePlayer(sender) ?: return 0
        controlSender.send(action, canonicalName(player), player.getUniqueId()) { result ->
            // A protected group continues in the player's BVC app; say where.
            val reply = (action as? ControlAction.JoinGroup)?.let { JoinReply.forResult(result, it.channel) }
            val message = reply ?: if (!result.ok) "[BVC] Action failed; try again" else null
            if (message != null) {
                onServerThread(Runnable { player.sendMessage(message) })
            }
        }
        player.sendMessage("BVC control sent")
        return 1
    }

    // The share code is the whole point of group create, so it is the reply.
    private fun createGroup(sender: CommandSender): Int {
        val player = requirePlayer(sender) ?: return 0
        controlSender.send(ControlAction.CreateGroup, canonicalName(player), player.getUniqueId()) { result ->
            val code = result.groupCode
            val message = if (result.ok && code != null) {
                "[BVC] Group created — share code: $code"
            } else {
                "[BVC] Group create failed; try again"
            }
            onServerThread(Runnable { player.sendMessage(message) })
        }
        return 1
    }

    private fun requirePlayer(sender: CommandSender): Player? {
        if (sender is Player) {
            return sender
        }
        sender.sendMessage("This command must be run by a player")
        return null
    }

    // Console only: from chat, anyone who gains op could promote themselves. RCON is not
    // the console, as on Paper.
    private fun admin(sender: CommandSender, action: String, gamertag: String): Int {
        if (sender !is ConsoleCommandSender || sender is RemoteConsoleCommandSender) {
            sender.sendMessage("/bvc admin can only be run from the server console")
            return 0
        }
        val adminAction = AdminAction.entries.firstOrNull { it.value == action } ?: return 0
        sender.sendMessage(adminConsole.run(adminAction, gamertag))
        return 1
    }

    companion object {
        private const val DISC = "disc"
        private const val GIVE = "give"
        private const val MUTE = "mute"
        private const val DEAFEN = "deafen"
        private const val RECORD = "record"
        private const val VOLUME = "volume"
        private const val HEAR = "hear"
        private const val JUKEBOX_MUTE = "jukebox_mute"
        private const val JUKEBOX_VOLUME = "jukebox_volume"
        private const val GROUP_CREATE = "group_create"
        private const val GROUP_JOIN = "group_join"
        private const val GROUP_LEAVE = "group_leave"
        private const val ADMIN = "admin"

        private const val DISC_PERMISSION = "bvc.disc"
        private const val GIVE_PERMISSION = "bvc.disc.give"

        private fun literal(name: String): CommandParameter = CommandParameter.newEnum(name, arrayOf(name))

        private fun toggle(): CommandParameter = CommandParameter.newEnum("on", CommandEnum.ENUM_BOOLEAN)

        private fun level(): CommandParameter = CommandParameter.newType("level", CommandParamType.INT)

        // Matched against online names here rather than as a selector, as Paper does.
        private fun playerName(): CommandParameter =
            CommandParameter.newType("player", CommandParamType.SELECTION, StringNode())
    }
}
