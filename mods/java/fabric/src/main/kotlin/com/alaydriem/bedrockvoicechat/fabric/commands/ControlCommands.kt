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
import net.minecraft.network.chat.ClickEvent
import net.minecraft.network.chat.Component
import net.minecraft.network.chat.HoverEvent
import net.minecraft.server.level.ServerPlayer

object ControlCommands {
    // The actor id must be the CANONICAL name — the same identity the position
    // path registers (Floodgate prefixes stripped). The raw profile name of a
    // prefixed Floodgate player has no alias row on the server, so it would
    // route nowhere.
    fun register(sender: ControlSender, canonicalName: (ServerPlayer) -> String) {
        CommandRegistrationCallback.EVENT.register { dispatcher, _, _ ->
            // Brigadier merges children into the existing "bvc" root registered by
            // DiscCommand, so these become subcommands alongside /bvc disc.
            dispatcher.register(
                Commands.literal("bvc")
                    .then(
                        Commands.literal("mute").then(
                            Commands.argument("on", BoolArgumentType.bool()).executes { ctx ->
                                dispatch(ctx, sender, canonicalName, ControlAction.Mute(BoolArgumentType.getBool(ctx, "on")))
                            }
                        )
                    )
                    .then(
                        Commands.literal("deafen").then(
                            Commands.argument("on", BoolArgumentType.bool()).executes { ctx ->
                                dispatch(ctx, sender, canonicalName, ControlAction.Deafen(BoolArgumentType.getBool(ctx, "on")))
                            }
                        )
                    )
                    .then(
                        Commands.literal("record").then(
                            Commands.argument("on", BoolArgumentType.bool()).executes { ctx ->
                                dispatch(ctx, sender, canonicalName, ControlAction.Record(BoolArgumentType.getBool(ctx, "on")))
                            }
                        )
                    )
                    .then(
                        Commands.literal("volume").then(
                            Commands.argument("player", StringArgumentType.string()).then(
                                Commands.argument("level", IntegerArgumentType.integer(0, 100)).executes { ctx ->
                                    dispatch(
                                        ctx, sender, canonicalName,
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
                                        ctx, sender, canonicalName,
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
                                    dispatchCreateGroup(ctx, sender, canonicalName)
                                }
                            )
                            .then(
                                Commands.literal("join").then(
                                    Commands.argument("code", StringArgumentType.string()).executes { ctx ->
                                        dispatch(
                                            ctx, sender, canonicalName,
                                            ControlAction.JoinGroup(StringArgumentType.getString(ctx, "code"))
                                        )
                                    }
                                )
                            )
                            .then(
                                Commands.literal("leave").executes { ctx ->
                                    dispatch(ctx, sender, canonicalName, ControlAction.LeaveGroup)
                                }
                            )
                    )
            )
        }
    }

    private fun dispatch(
        ctx: CommandContext<CommandSourceStack>,
        sender: ControlSender,
        canonicalName: (ServerPlayer) -> String,
        action: ControlAction
    ): Int {
        val player = ctx.source.player
        if (player == null) {
            ctx.source.sendFailure(Component.literal("This command must be run by a player"))
            return 0
        }
        // The result callback can arrive on the HTTP executor thread; hop back to
        // the server thread before touching the player.
        val server = ctx.source.server
        sender.send(action, canonicalName(player)) { result ->
            if (!result.ok) {
                server.execute {
                    player.sendSystemMessage(Component.literal("[BVC] Action failed; try again"))
                }
            }
        }
        ctx.source.sendSuccess({ Component.literal("BVC control sent") }, false)
        return Command.SINGLE_SUCCESS
    }

    // The share code is the whole point of group create: reply with it as a
    // click-to-copy component instead of the generic acknowledgement.
    private fun dispatchCreateGroup(
        ctx: CommandContext<CommandSourceStack>,
        sender: ControlSender,
        canonicalName: (ServerPlayer) -> String
    ): Int {
        val player = ctx.source.player
        if (player == null) {
            ctx.source.sendFailure(Component.literal("This command must be run by a player"))
            return 0
        }
        val server = ctx.source.server
        sender.send(ControlAction.CreateGroup, canonicalName(player)) { result ->
            val code = result.groupCode
            server.execute {
                if (result.ok && code != null) {
                    player.sendSystemMessage(
                        Component.literal("[BVC] Group created — share code: ").append(
                            Component.literal(code).withStyle { style ->
                                style
                                    .withClickEvent(ClickEvent.CopyToClipboard(code))
                                    .withHoverEvent(
                                        HoverEvent.ShowText(Component.literal("Click to copy"))
                                    )
                            }
                        )
                    )
                } else {
                    player.sendSystemMessage(
                        Component.literal("[BVC] Group create failed; try again")
                    )
                }
            }
        }
        return Command.SINGLE_SUCCESS
    }
}
