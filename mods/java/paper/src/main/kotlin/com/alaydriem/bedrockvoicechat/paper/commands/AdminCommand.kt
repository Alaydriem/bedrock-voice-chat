package com.alaydriem.bedrockvoicechat.paper.commands

import com.alaydriem.bedrockvoicechat.admin.AdminAction
import com.alaydriem.bedrockvoicechat.admin.AdminConsole
import com.mojang.brigadier.Command
import com.mojang.brigadier.arguments.StringArgumentType
import com.mojang.brigadier.builder.LiteralArgumentBuilder
import io.papermc.paper.command.brigadier.CommandSourceStack
import io.papermc.paper.command.brigadier.Commands
import net.kyori.adventure.text.Component
import org.bukkit.command.ConsoleCommandSender

/**
 * `/bvc admin grant|revoke|deny <gamertag>`. The gamertag is the one the player signs in to
 * BVC with, matched verbatim; quote it when it contains spaces.
 */
@Suppress("UnstableApiUsage")
class AdminCommand(private val console: AdminConsole) {

    fun addTo(bvc: LiteralArgumentBuilder<CommandSourceStack>) {
        // Console only: from chat, anyone who gains op could promote themselves.
        val admin = Commands.literal("admin").requires { it.sender is ConsoleCommandSender }
        for (action in AdminAction.entries) {
            admin.then(
                Commands.literal(action.value).then(
                    Commands.argument("gamertag", StringArgumentType.string()).executes { ctx ->
                        val gamertag = StringArgumentType.getString(ctx, "gamertag")
                        ctx.source.sender.sendMessage(Component.text(console.run(action, gamertag)))
                        Command.SINGLE_SUCCESS
                    }
                )
            )
        }
        bvc.then(admin)
    }
}
