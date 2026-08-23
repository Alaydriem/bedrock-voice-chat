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
import net.kyori.adventure.text.event.ClickEvent
import net.kyori.adventure.text.event.HoverEvent
import org.bukkit.Bukkit
import org.bukkit.entity.Player

// The actor id must be the CANONICAL name — the same identity the position path
// registers (Floodgate prefixes stripped). The raw executor name of a prefixed
// Floodgate player has no alias row on the server, so it would route nowhere.
@Suppress("UnstableApiUsage")
class ControlCommands(
    private val sender: ControlSender,
    private val canonicalName: (Player) -> String,
) {

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
                        Commands.argument("level", IntegerArgumentType.integer(0, ControlAction.MAX_LEVEL)).executes { ctx ->
                            dispatchTargeted(ctx, StringArgumentType.getString(ctx, "player")) { target ->
                                ControlAction.Volume(target, IntegerArgumentType.getInteger(ctx, "level"))
                            }
                        }
                    )
                )
            )
            .then(
                Commands.literal("hear").then(
                    Commands.argument("player", StringArgumentType.string()).then(
                        Commands.argument("on", BoolArgumentType.bool()).executes { ctx ->
                            dispatchTargeted(ctx, StringArgumentType.getString(ctx, "player")) { target ->
                                ControlAction.Hear(target, BoolArgumentType.getBool(ctx, "on"))
                            }
                        }
                    )
                )
            )
            // Neither jukebox subcommand takes a player argument, so `dispatchTargeted` and its
            // canonical-name resolution are correctly not on this path.
            .then(
                Commands.literal("jukebox")
                    .then(
                        Commands.literal("mute").then(
                            Commands.argument("on", BoolArgumentType.bool()).executes { ctx ->
                                // The plane carries "heard", the command reads as "mute".
                                dispatch(
                                    ctx,
                                    ControlAction.Hear(
                                        ControlAction.JUKEBOX_TARGET,
                                        !BoolArgumentType.getBool(ctx, "on"),
                                    ),
                                )
                            }
                        )
                    )
                    .then(
                        Commands.literal("volume").then(
                            Commands.argument("level", IntegerArgumentType.integer(0, ControlAction.MAX_LEVEL)).executes { ctx ->
                                dispatch(
                                    ctx,
                                    ControlAction.Volume(
                                        ControlAction.JUKEBOX_TARGET,
                                        IntegerArgumentType.getInteger(ctx, "level"),
                                    ),
                                )
                            }
                        )
                    )
            )
            .then(
                Commands.literal("group")
                    .then(
                        Commands.literal("create").executes { ctx ->
                            dispatchCreateGroup(ctx)
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

    // Per-player preferences key on the target's CANONICAL gamertag everywhere
    // downstream (the desktop gain store, the sink's name remap, the player
    // cards) — like the actor, a typed target must resolve to a real online
    // player and pass through the same canonical-name mapping, never raw text.
    private fun dispatchTargeted(
        ctx: CommandContext<CommandSourceStack>,
        typed: String,
        build: (String) -> ControlAction,
    ): Int {
        val resolved = Bukkit.getOnlinePlayers().firstOrNull { it.name.equals(typed, ignoreCase = true) }
        if (resolved == null) {
            ctx.source.sender.sendMessage(Component.text("[BVC] Unknown player: $typed"))
            return 0
        }
        return dispatch(ctx, build(canonicalName(resolved)))
    }

    private fun dispatch(ctx: CommandContext<CommandSourceStack>, action: ControlAction): Int {
        val executor = ctx.source.sender
        if (executor !is Player) {
            executor.sendMessage(Component.text("This command must be run by a player"))
            return 0
        }
        // Adventure audiences are thread-safe, so replying from the HTTP
        // executor thread is fine.
        sender.send(action, canonicalName(executor)) { result ->
            if (!result.ok) {
                executor.sendMessage(Component.text("[BVC] Action failed; try again"))
            }
        }
        executor.sendMessage(Component.text("BVC control sent"))
        return Command.SINGLE_SUCCESS
    }

    // The share code is the whole point of group create: reply with it as a
    // click-to-copy component instead of the generic acknowledgement.
    private fun dispatchCreateGroup(ctx: CommandContext<CommandSourceStack>): Int {
        val executor = ctx.source.sender
        if (executor !is Player) {
            executor.sendMessage(Component.text("This command must be run by a player"))
            return 0
        }
        sender.send(ControlAction.CreateGroup, canonicalName(executor)) { result ->
            val code = result.groupCode
            if (result.ok && code != null) {
                executor.sendMessage(
                    Component.text("[BVC] Group created — share code: ")
                        .append(
                            Component.text(code)
                                .clickEvent(ClickEvent.copyToClipboard(code))
                                .hoverEvent(HoverEvent.showText(Component.text("Click to copy")))
                        )
                )
            } else {
                executor.sendMessage(Component.text("[BVC] Group create failed; try again"))
            }
        }
        return Command.SINGLE_SUCCESS
    }
}
