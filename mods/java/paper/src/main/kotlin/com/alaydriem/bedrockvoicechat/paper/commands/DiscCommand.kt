package com.alaydriem.bedrockvoicechat.paper.commands

import com.mojang.brigadier.Command
import com.mojang.brigadier.arguments.StringArgumentType
import io.papermc.paper.command.brigadier.Commands
import io.papermc.paper.command.brigadier.argument.ArgumentTypes
import io.papermc.paper.command.brigadier.argument.resolvers.selector.PlayerSelectorArgumentResolver
import io.papermc.paper.datacomponent.DataComponentTypes
import io.papermc.paper.plugin.lifecycle.event.types.LifecycleEvents
import net.kyori.adventure.text.Component
import org.bukkit.Material
import org.bukkit.NamespacedKey
import org.bukkit.entity.Player
import org.bukkit.inventory.ItemStack
import org.bukkit.persistence.PersistentDataType
import org.bukkit.plugin.java.JavaPlugin

@Suppress("UnstableApiUsage")
class DiscCommand(private val plugin: JavaPlugin) {

    private val isBvcDiscKey = NamespacedKey(plugin, "is_bvc_disc")
    private val audioIdKey = NamespacedKey(plugin, "audio_id")

    fun register() {
        plugin.lifecycleManager.registerEventHandler(LifecycleEvents.COMMANDS) { event ->
            event.registrar().register(
                Commands.literal("bvc")
                    .then(
                        Commands.literal("disc")
                            .requires { it.sender.hasPermission("bvc.disc") }
                            .then(
                                Commands.argument("audio_id", StringArgumentType.greedyString())
                                    .executes { ctx ->
                                        val audioId = StringArgumentType.getString(ctx, "audio_id")
                                        val sender = ctx.source.sender
                                        if (sender !is Player) {
                                            sender.sendMessage(
                                                Component.text(
                                                    "/bvc disc must be run by a player. Use /bvc give <player> <audio_id> from the console."
                                                )
                                            )
                                            return@executes 0
                                        }
                                        giveDisc(sender, audioId)
                                        sender.sendMessage(Component.text("Gave BVC audio disc: $audioId"))
                                        Command.SINGLE_SUCCESS
                                    }
                            )
                    )
                    .then(
                        Commands.literal("give")
                            .requires { it.sender.hasPermission("bvc.disc.give") }
                            .then(
                                Commands.argument("target", ArgumentTypes.player())
                                    .then(
                                        Commands.argument("audio_id", StringArgumentType.greedyString())
                                            .executes { ctx ->
                                                val audioId = StringArgumentType.getString(ctx, "audio_id")
                                                val resolver = ctx.getArgument(
                                                    "target", PlayerSelectorArgumentResolver::class.java
                                                )
                                                val targets = resolver.resolve(ctx.source)
                                                if (targets.isEmpty()) {
                                                    ctx.source.sender.sendMessage(
                                                        Component.text("No matching player found")
                                                    )
                                                    return@executes 0
                                                }
                                                for (target in targets) {
                                                    giveDisc(target, audioId)
                                                }
                                                ctx.source.sender.sendMessage(
                                                    Component.text(
                                                        "Gave BVC audio disc '$audioId' to ${targets.size} player(s)"
                                                    )
                                                )
                                                Command.SINGLE_SUCCESS
                                            }
                                    )
                            )
                    )
                    .build(),
                "Bedrock Voice Chat commands"
            )
        }
    }

    private fun giveDisc(player: Player, audioId: String) {
        val disc = ItemStack(Material.MUSIC_DISC_5)
        disc.editMeta { meta ->
            meta.displayName(Component.text("BVC: $audioId"))
            meta.persistentDataContainer.set(isBvcDiscKey, PersistentDataType.BOOLEAN, true)
            meta.persistentDataContainer.set(audioIdKey, PersistentDataType.STRING, audioId)
        }
        disc.unsetData(DataComponentTypes.JUKEBOX_PLAYABLE)
        if (player.inventory.addItem(disc).isNotEmpty()) {
            player.world.dropItem(player.location, disc)
        }
    }
}
