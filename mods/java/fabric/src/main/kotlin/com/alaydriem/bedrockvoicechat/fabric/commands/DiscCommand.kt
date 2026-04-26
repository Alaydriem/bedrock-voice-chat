package com.alaydriem.bedrockvoicechat.fabric.commands

import com.alaydriem.bedrockvoicechat.fabric.audio.JukeboxListener
import com.mojang.brigadier.Command
import com.mojang.brigadier.arguments.StringArgumentType
import net.fabricmc.fabric.api.command.v2.CommandRegistrationCallback
import net.minecraft.command.argument.EntityArgumentType
import net.minecraft.command.permission.Permission
import net.minecraft.command.permission.PermissionLevel
import net.minecraft.component.DataComponentTypes
import net.minecraft.server.command.CommandManager
import net.minecraft.text.Text

object DiscCommand {
    fun register() {
        CommandRegistrationCallback.EVENT.register { dispatcher, _, _ ->
            dispatcher.register(
                CommandManager.literal("bvc")
                    .then(
                        CommandManager.literal("disc")
                            .then(
                                CommandManager.argument("audio_id", StringArgumentType.greedyString())
                                    .executes { ctx ->
                                        val audioId = StringArgumentType.getString(ctx, "audio_id")
                                        val player = ctx.source.player
                                        if (player == null) {
                                            ctx.source.sendError(
                                                Text.literal(
                                                    "/bvc disc must be run by a player. Use /bvc give <player> <audio_id> from the console."
                                                )
                                            )
                                            return@executes 0
                                        }

                                        val disc = JukeboxListener.createBvcDisc(audioId)
                                        disc.set(DataComponentTypes.CUSTOM_NAME, Text.literal("BVC: $audioId"))

                                        if (!player.inventory.insertStack(disc)) {
                                            player.dropItem(disc, false)
                                        }

                                        ctx.source.sendFeedback(
                                            { Text.literal("Gave BVC audio disc: $audioId") },
                                            true
                                        )

                                        Command.SINGLE_SUCCESS
                                    }
                            )
                    )
                    .then(
                        CommandManager.literal("give")
                            .requires { it.permissions.hasPermission(Permission.Level(PermissionLevel.GAMEMASTERS)) }
                            .then(
                                CommandManager.argument("target", EntityArgumentType.players())
                                    .then(
                                        CommandManager.argument("audio_id", StringArgumentType.greedyString())
                                            .executes { ctx ->
                                                val audioId = StringArgumentType.getString(ctx, "audio_id")
                                                val targets = EntityArgumentType.getPlayers(ctx, "target")
                                                if (targets.isEmpty()) {
                                                    ctx.source.sendError(Text.literal("No matching player found"))
                                                    return@executes 0
                                                }

                                                for (target in targets) {
                                                    val disc = JukeboxListener.createBvcDisc(audioId)
                                                    disc.set(
                                                        DataComponentTypes.CUSTOM_NAME,
                                                        Text.literal("BVC: $audioId")
                                                    )
                                                    if (!target.inventory.insertStack(disc)) {
                                                        target.dropItem(disc, false)
                                                    }
                                                }

                                                ctx.source.sendFeedback(
                                                    {
                                                        Text.literal(
                                                            "Gave BVC audio disc '$audioId' to ${targets.size} player(s)"
                                                        )
                                                    },
                                                    true
                                                )

                                                Command.SINGLE_SUCCESS
                                            }
                                    )
                            )
                    )
            )
        }
    }
}
