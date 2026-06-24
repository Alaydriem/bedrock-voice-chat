package com.alaydriem.bedrockvoicechat.fabric.commands

import com.alaydriem.bedrockvoicechat.fabric.audio.JukeboxListener
import com.mojang.brigadier.Command
import com.mojang.brigadier.arguments.StringArgumentType
import net.fabricmc.fabric.api.command.v2.CommandRegistrationCallback
import net.minecraft.commands.Commands
import net.minecraft.commands.arguments.EntityArgument
import net.minecraft.core.component.DataComponents
import net.minecraft.network.chat.Component
import net.minecraft.server.permissions.Permission
import net.minecraft.server.permissions.PermissionLevel

object DiscCommand {
    fun register() {
        CommandRegistrationCallback.EVENT.register { dispatcher, _, _ ->
            dispatcher.register(
                Commands.literal("bvc")
                    .then(
                        Commands.literal("disc")
                            .then(
                                Commands.argument("audio_id", StringArgumentType.greedyString())
                                    .executes { ctx ->
                                        val audioId = StringArgumentType.getString(ctx, "audio_id")
                                        val player = ctx.source.player
                                        if (player == null) {
                                            ctx.source.sendFailure(
                                                Component.literal(
                                                    "/bvc disc must be run by a player. Use /bvc give <player> <audio_id> from the console."
                                                )
                                            )
                                            return@executes 0
                                        }

                                        val disc = JukeboxListener.createBvcDisc(audioId)
                                        disc.set(DataComponents.CUSTOM_NAME, Component.literal("BVC: $audioId"))

                                        if (!player.inventory.add(disc)) {
                                            player.drop(disc, false)
                                        }

                                        ctx.source.sendSuccess(
                                            { Component.literal("Gave BVC audio disc: $audioId") },
                                            true
                                        )

                                        Command.SINGLE_SUCCESS
                                    }
                            )
                    )
                    .then(
                        Commands.literal("give")
                            .requires { it.permissions().hasPermission(Permission.HasCommandLevel(PermissionLevel.GAMEMASTERS)) }
                            .then(
                                Commands.argument("target", EntityArgument.players())
                                    .then(
                                        Commands.argument("audio_id", StringArgumentType.greedyString())
                                            .executes { ctx ->
                                                val audioId = StringArgumentType.getString(ctx, "audio_id")
                                                val targets = EntityArgument.getPlayers(ctx, "target")
                                                if (targets.isEmpty()) {
                                                    ctx.source.sendFailure(Component.literal("No matching player found"))
                                                    return@executes 0
                                                }

                                                for (target in targets) {
                                                    val disc = JukeboxListener.createBvcDisc(audioId)
                                                    disc.set(
                                                        DataComponents.CUSTOM_NAME,
                                                        Component.literal("BVC: $audioId")
                                                    )
                                                    if (!target.inventory.add(disc)) {
                                                        target.drop(disc, false)
                                                    }
                                                }

                                                ctx.source.sendSuccess(
                                                    {
                                                        Component.literal(
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
