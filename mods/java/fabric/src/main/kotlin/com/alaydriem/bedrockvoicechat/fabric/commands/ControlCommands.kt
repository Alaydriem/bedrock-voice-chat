package com.alaydriem.bedrockvoicechat.fabric.commands

import com.alaydriem.bedrockvoicechat.control.ControlAction
import com.alaydriem.bedrockvoicechat.control.ControlSender
import com.mojang.brigadier.Command
import com.mojang.brigadier.arguments.BoolArgumentType
import com.mojang.brigadier.arguments.IntegerArgumentType
import com.mojang.brigadier.arguments.StringArgumentType
import com.mojang.brigadier.context.CommandContext
import net.fabricmc.fabric.api.command.v2.CommandRegistrationCallback
import net.minecraft.commands.CommandSourceStack
import net.minecraft.commands.Commands
import net.minecraft.network.chat.Component

object ControlCommands {
    fun register(sender: ControlSender) {
        CommandRegistrationCallback.EVENT.register { dispatcher, _, _ ->
            // Brigadier merges children into the existing "bvc" root registered by
            // DiscCommand, so these become subcommands alongside /bvc disc.
            dispatcher.register(
                Commands.literal("bvc")
                    .then(
                        Commands.literal("mute").then(
                            Commands.argument("on", BoolArgumentType.bool()).executes { ctx ->
                                dispatch(ctx, sender, ControlAction.Mute(BoolArgumentType.getBool(ctx, "on")))
                            }
                        )
                    )
                    .then(
                        Commands.literal("deafen").then(
                            Commands.argument("on", BoolArgumentType.bool()).executes { ctx ->
                                dispatch(ctx, sender, ControlAction.Deafen(BoolArgumentType.getBool(ctx, "on")))
                            }
                        )
                    )
                    .then(
                        Commands.literal("record").then(
                            Commands.argument("on", BoolArgumentType.bool()).executes { ctx ->
                                dispatch(ctx, sender, ControlAction.Record(BoolArgumentType.getBool(ctx, "on")))
                            }
                        )
                    )
                    .then(
                        Commands.literal("volume").then(
                            Commands.argument("player", StringArgumentType.string()).then(
                                Commands.argument("level", IntegerArgumentType.integer(0, 100)).executes { ctx ->
                                    dispatch(
                                        ctx, sender,
                                        ControlAction.Volume(
                                            StringArgumentType.getString(ctx, "player"),
                                            IntegerArgumentType.getInteger(ctx, "level")
                                        )
                                    )
                                }
                            )
                        )
                    )
                    .then(
                        Commands.literal("hear").then(
                            Commands.argument("player", StringArgumentType.string()).then(
                                Commands.argument("on", BoolArgumentType.bool()).executes { ctx ->
                                    dispatch(
                                        ctx, sender,
                                        ControlAction.Hear(
                                            StringArgumentType.getString(ctx, "player"),
                                            BoolArgumentType.getBool(ctx, "on")
                                        )
                                    )
                                }
                            )
                        )
                    )
                    .then(
                        Commands.literal("group")
                            .then(
                                Commands.literal("create").executes { ctx ->
                                    dispatch(ctx, sender, ControlAction.CreateGroup)
                                }
                            )
                            .then(
                                Commands.literal("join").then(
                                    Commands.argument("code", StringArgumentType.string()).executes { ctx ->
                                        dispatch(
                                            ctx, sender,
                                            ControlAction.JoinGroup(StringArgumentType.getString(ctx, "code"))
                                        )
                                    }
                                )
                            )
                            .then(
                                Commands.literal("leave").executes { ctx ->
                                    dispatch(ctx, sender, ControlAction.LeaveGroup)
                                }
                            )
                    )
            )
        }
    }

    private fun dispatch(
        ctx: CommandContext<CommandSourceStack>,
        sender: ControlSender,
        action: ControlAction
    ): Int {
        val player = ctx.source.player
        if (player == null) {
            ctx.source.sendFailure(Component.literal("This command must be run by a player"))
            return 0
        }
        sender.send(action, player.gameProfile.name)
        ctx.source.sendSuccess({ Component.literal("BVC control sent") }, false)
        return Command.SINGLE_SUCCESS
    }
}
