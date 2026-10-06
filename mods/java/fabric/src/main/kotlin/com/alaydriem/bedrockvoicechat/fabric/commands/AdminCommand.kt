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
 * `/bvc admin grant|revoke|deny <gamertag>`. The gamertag is the one the player signs in to
 * BVC with, matched verbatim; quote it when it contains spaces.
 */
object AdminCommand {
    fun register(console: AdminConsole) {
        CommandRegistrationCallback.EVENT.register { dispatcher, _, _ ->
            // A command block also has no player, but runs at GAMEMASTERS. Only the console
            // and RCON hold OWNERS, so both checks are needed to keep this off the map.
            val admin = Commands.literal("admin").requires {
                it.player == null &&
                    it.permissions().hasPermission(Permission.HasCommandLevel(PermissionLevel.OWNERS))
            }
            for (action in AdminAction.entries) {
                admin.then(
                    Commands.literal(action.value).then(
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
            // Brigadier merges this into the "bvc" root DiscCommand registers.
            dispatcher.register(Commands.literal("bvc").then(admin))
        }
    }
}
