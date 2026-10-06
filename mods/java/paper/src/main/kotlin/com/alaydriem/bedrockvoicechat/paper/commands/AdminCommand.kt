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
 * `/bvc admin grant|revoke|deny <gamertag>` — changes a player's BVC `admin` permission on
 * the embedded server.
 *
 * The gamertag is the one the player signs in to BVC with, used verbatim; quote it when it
 * contains spaces.
 */
@Suppress("UnstableApiUsage")
class AdminCommand(private val console: AdminConsole) {

    // Contributed to the shared "bvc" root so there is exactly one registration of it.
    fun addTo(bvc: LiteralArgumentBuilder<CommandSourceStack>) {
        // Granting admin from chat would let anyone who gains op promote themselves. The
        // console is the one place an operator already trusts with the server itself.
        val admin = Commands.literal("admin").requires { it.sender is ConsoleCommandSender }
        for (action in AdminAction.entries) {
            admin.then(
                Commands.literal(action.wire).then(
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
