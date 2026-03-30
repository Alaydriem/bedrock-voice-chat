package com.alaydriem.bedrockvoicechat.fabric.commands

import com.alaydriem.bedrockvoicechat.fabric.audio.JukeboxListener
import com.mojang.brigadier.Command
import com.mojang.brigadier.arguments.StringArgumentType
import net.fabricmc.fabric.api.command.v2.CommandRegistrationCallback
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
                            .requires { it.permissions.hasPermission(Permission.Level(PermissionLevel.GAMEMASTERS)) }
                            .then(
                                CommandManager.argument("audio_id", StringArgumentType.string())
                                    .executes { ctx ->
                                        val audioId = StringArgumentType.getString(ctx, "audio_id")
                                        val player = ctx.source.playerOrThrow

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
            )
        }
    }
}
