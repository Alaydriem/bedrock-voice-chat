package com.alaydriem.bedrockvoicechat.fabric.commands

import com.alaydriem.bedrockvoicechat.admin.AdminAction
import com.alaydriem.bedrockvoicechat.admin.AdminConsole
import com.mojang.brigadier.Command
import com.mojang.brigadier.arguments.StringArgumentType
import net.fabricmc.fabric.api.command.v2.CommandRegistrationCallback
import net.minecraft.commands.Commands
import net.minecraft.network.chat.Component
import net.minecraft.server.permissions.Permission
import net.minecraft.server.permissions.PermissionLevel

/**
 * `/bvc admin grant|revoke|deny <gamertag>` — changes a player's BVC `admin` permission on
 * the embedded server.
 *
 * The gamertag is the one the player signs in to BVC with, used verbatim; quote it when it
 * contains spaces.
 */
object AdminCommand {
    fun register(console: AdminConsole) {
        CommandRegistrationCallback.EVENT.register { dispatcher, _, _ ->
            // A source with no player is not necessarily the console: a command block has
            // none either. Command blocks run at GAMEMASTERS; only the console and RCON hold
            // OWNERS, so both conditions are needed to keep admin grants off the map.
            val admin = Commands.literal("admin").requires {
                it.player == null &&
                    it.permissions().hasPermission(Permission.HasCommandLevel(PermissionLevel.OWNERS))
            }
            for (action in AdminAction.entries) {
                admin.then(
                    Commands.literal(action.wire).then(
                        Commands.argument("gamertag", StringArgumentType.string()).executes { ctx ->
                            val gamertag = StringArgumentType.getString(ctx, "gamertag")
                            ctx.source.sendSystemMessage(
                                Component.literal(console.run(action, gamertag))
                            )
                            Command.SINGLE_SUCCESS
                        }
                    )
                )
            }
            // Brigadier merges children into the existing "bvc" root registered by
            // DiscCommand, so this becomes a subcommand alongside /bvc disc.
            dispatcher.register(Commands.literal("bvc").then(admin))
        }
    }
}
