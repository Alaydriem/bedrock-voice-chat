package com.alaydriem.bedrockvoicechat.paper.commands

import com.alaydriem.bedrockvoicechat.control.ControlAction
import com.alaydriem.bedrockvoicechat.control.ControlSender
import com.mojang.brigadier.Command
import com.mojang.brigadier.arguments.BoolArgumentType
import com.mojang.brigadier.arguments.IntegerArgumentType
import com.mojang.brigadier.arguments.StringArgumentType
import com.mojang.brigadier.builder.LiteralArgumentBuilder
import com.mojang.brigadier.context.CommandContext
import io.papermc.paper.command.brigadier.CommandSourceStack
import io.papermc.paper.command.brigadier.Commands
import net.kyori.adventure.text.Component
import org.bukkit.entity.Player

@Suppress("UnstableApiUsage")
class ControlCommands(private val sender: ControlSender) {

    // Contribute the control subcommands to a shared "bvc" root so there is exactly
    // one registration of the root (DiscCommand contributes disc/give to the same
    // builder). This avoids relying on Paper's registrar merging duplicate roots.
    fun addTo(bvc: LiteralArgumentBuilder<CommandSourceStack>) {
        bvc
            .then(
                Commands.literal("mute").then(
                    Commands.argument("on", BoolArgumentType.bool()).executes { ctx ->
                        dispatch(ctx, ControlAction.Mute(BoolArgumentType.getBool(ctx, "on")))
                    }
                )
            )
            .then(
                Commands.literal("deafen").then(
                    Commands.argument("on", BoolArgumentType.bool()).executes { ctx ->
                        dispatch(ctx, ControlAction.Deafen(BoolArgumentType.getBool(ctx, "on")))
                    }
                )
            )
            .then(
                Commands.literal("record").then(
                    Commands.argument("on", BoolArgumentType.bool()).executes { ctx ->
                        dispatch(ctx, ControlAction.Record(BoolArgumentType.getBool(ctx, "on")))
                    }
                )
            )
            .then(
                Commands.literal("volume").then(
                    Commands.argument("player", StringArgumentType.string()).then(
                        Commands.argument("level", IntegerArgumentType.integer(0, 100)).executes { ctx ->
                            dispatch(
                                ctx,
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
                                ctx,
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
                            dispatch(ctx, ControlAction.CreateGroup)
                        }
                    )
                    .then(
                        Commands.literal("join").then(
                            Commands.argument("code", StringArgumentType.string()).executes { ctx ->
                                dispatch(ctx, ControlAction.JoinGroup(StringArgumentType.getString(ctx, "code")))
                            }
                        )
                    )
                    .then(
                        Commands.literal("leave").executes { ctx ->
                            dispatch(ctx, ControlAction.LeaveGroup)
                        }
                    )
            )
    }

    private fun dispatch(ctx: CommandContext<CommandSourceStack>, action: ControlAction): Int {
        val executor = ctx.source.sender
        if (executor !is Player) {
            executor.sendMessage(Component.text("This command must be run by a player"))
            return 0
        }
        sender.send(action, executor.name)
        executor.sendMessage(Component.text("BVC control sent"))
        return Command.SINGLE_SUCCESS
    }
}
