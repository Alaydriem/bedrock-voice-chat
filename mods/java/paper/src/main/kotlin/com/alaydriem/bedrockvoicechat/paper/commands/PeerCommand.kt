package com.alaydriem.bedrockvoicechat.paper.commands

import com.alaydriem.bedrockvoicechat.svc.PairingOutcome
import com.alaydriem.bedrockvoicechat.svc.PairingRequest
import com.mojang.brigadier.Command
import com.mojang.brigadier.arguments.StringArgumentType
import com.mojang.brigadier.builder.LiteralArgumentBuilder
import io.papermc.paper.command.brigadier.CommandSourceStack
import io.papermc.paper.command.brigadier.Commands
import net.kyori.adventure.text.Component
import org.bukkit.command.ConsoleCommandSender

/**
 * `/bvc peer <code>` — redeems a pairing code against the external BVC server.
 *
 * `request` is a supplier rather than a value because the bridge is not built until Simple
 * Voice Chat hands over its server API, which happens after commands are registered.
 */
@Suppress("UnstableApiUsage")
class PeerCommand(private val request: () -> PairingRequest?) {

    // Contributed to the shared "bvc" root so there is exactly one registration of it.
    fun addTo(bvc: LiteralArgumentBuilder<CommandSourceStack>) {
        bvc.then(
            Commands.literal("peer")
                // A pairing code typed in chat reaches the chat log and every player with
                // permission to read it. Restricting to the console keeps the secret in the
                // one place an operator already trusts with credentials.
                .requires { it.sender is ConsoleCommandSender }
                .then(
                    Commands.argument("code", StringArgumentType.word()).executes { ctx ->
                        val pairing = request()
                        if (pairing == null) {
                            ctx.source.sender.sendMessage(
                                Component.text(
                                    "Simple Voice Chat peering is not available on this server."
                                )
                            )
                            return@executes Command.SINGLE_SUCCESS
                        }

                        val code = StringArgumentType.getString(ctx, "code")
                        ctx.source.sender.sendMessage(Component.text(message(pairing.submit(code))))

                        Command.SINGLE_SUCCESS
                    }
                )
        )
    }

    private fun message(outcome: PairingOutcome): String = when (outcome) {
        PairingOutcome.Paired -> "Paired with the BVC server. The bridge is connecting."
        PairingOutcome.AlreadyPaired -> "This bridge was already paired."
        PairingOutcome.WrongCode ->
            "That code was not accepted. Mint a new one with `bvc-server relay pair`."
        PairingOutcome.Expired ->
            "That code has expired. Mint a new one with `bvc-server relay pair`."
        PairingOutcome.Unreachable -> "Could not reach the BVC server's peer endpoint."
        PairingOutcome.NotEligible ->
            "This BVC server does not have peering enabled. Set `peering = true` in its config.hcl."
        PairingOutcome.EmbeddedMode -> "This server pairs automatically. No code is needed."
    }
}
