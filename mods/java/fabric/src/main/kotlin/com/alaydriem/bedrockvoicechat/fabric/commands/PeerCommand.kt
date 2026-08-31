package com.alaydriem.bedrockvoicechat.fabric.commands

import com.alaydriem.bedrockvoicechat.svc.PairingOutcome
import com.alaydriem.bedrockvoicechat.svc.PairingRequest
import com.mojang.brigadier.Command
import com.mojang.brigadier.arguments.StringArgumentType
import net.fabricmc.fabric.api.command.v2.CommandRegistrationCallback
import net.minecraft.commands.Commands
import net.minecraft.network.chat.Component

/**
 * `/bvc peer <code>` — redeems a pairing code against the external BVC server.
 *
 * `request` is a supplier rather than a value because the bridge is not built until Simple
 * Voice Chat hands over its server API, which happens after commands are registered.
 */
object PeerCommand {
    fun register(request: () -> PairingRequest?) {
        CommandRegistrationCallback.EVENT.register { dispatcher, _, _ ->
            // Brigadier merges children into the existing "bvc" root registered by
            // DiscCommand, so this becomes a subcommand alongside /bvc disc.
            dispatcher.register(
                Commands.literal("bvc")
                    .then(
                        Commands.literal("peer")
                            // A pairing code typed in chat reaches the chat log and every
                            // player with permission to read it. A source with no player is
                            // the console, which is the one place an operator already trusts
                            // with credentials.
                            .requires { it.player == null }
                            .then(
                                Commands.argument("code", StringArgumentType.word())
                                    .executes { ctx ->
                                        val pairing = request()
                                        if (pairing == null) {
                                            ctx.source.sendSystemMessage(
                                                Component.literal(
                                                    "Simple Voice Chat peering is not available on this server."
                                                )
                                            )
                                            return@executes Command.SINGLE_SUCCESS
                                        }

                                        val code = StringArgumentType.getString(ctx, "code")
                                        ctx.source.sendSystemMessage(
                                            Component.literal(message(pairing.submit(code)))
                                        )

                                        Command.SINGLE_SUCCESS
                                    }
                            )
                    )
            )
        }
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
