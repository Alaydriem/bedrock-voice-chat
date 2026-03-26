package com.alaydriem.bedrockvoicechat.fabric.commands

import com.alaydriem.bedrockvoicechat.fabric.audio.JukeboxListener
import com.mojang.brigadier.Command
import com.mojang.brigadier.arguments.StringArgumentType
import net.fabricmc.fabric.api.command.v2.CommandRegistrationCallback
import net.minecraft.commands.Commands
import net.minecraft.core.component.DataComponents
import net.minecraft.network.chat.Component
import net.minecraft.server.permissions.Permissions

object DiscCommand {
    fun register() {
        CommandRegistrationCallback.EVENT.register { dispatcher, _, _ ->
            dispatcher.register(
                Commands.literal("bvc")
                    .then(
                        Commands.literal("disc")
                            .requires { it.permissions().hasPermission(Permissions.COMMANDS_GAMEMASTER) }
                            .then(
                                Commands.argument("audio_id", StringArgumentType.string())
                                    .executes { ctx ->
                                        val audioId = StringArgumentType.getString(ctx, "audio_id")
                                        val player = ctx.source.getPlayerOrException()

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
            )
        }
    }
}
