package com.alaydriem.bedrockvoicechat.paper.audio

import org.bukkit.command.Command
import org.bukkit.command.CommandExecutor
import org.bukkit.command.CommandSender
import org.bukkit.entity.Player
import org.slf4j.LoggerFactory

/**
 * Handles the /bvc command for Paper.
 * Currently supports: /bvc disc <name_or_id>
 */
class AudioCommands(
    private val audioListener: AudioPlayerListener
) : CommandExecutor {

    companion object {
        private val logger = LoggerFactory.getLogger("BVC Commands")
    }

    override fun onCommand(sender: CommandSender, command: Command, label: String, args: Array<out String>): Boolean {
        if (sender !is Player) {
            sender.sendMessage("This command can only be used by players")
            return true
        }

        if (args.isEmpty()) {
            sender.sendMessage("Usage: /bvc:disc <uuid>")
            return true
        }

        when (args[0].lowercase()) {
            "disc" -> handleDiscCommand(sender, args)
            else -> {
                sender.sendMessage("Unknown subcommand: ${args[0]}")
                sender.sendMessage("Usage: /bvc:disc <uuid>")
            }
        }

        return true
    }

    private fun handleDiscCommand(player: Player, args: Array<out String>) {
        if (args.size < 2) {
            player.sendMessage("Usage: /bvc:disc <uuid>")
            return
        }

        val nameOrId = args.drop(1).joinToString(" ")

        val disc = audioListener.createBvcDisc(nameOrId, "BVC: $nameOrId")
        player.inventory.addItem(disc)
        logger.info("Gave {} a BVC audio disc: {}", player.name, nameOrId)
    }
}
