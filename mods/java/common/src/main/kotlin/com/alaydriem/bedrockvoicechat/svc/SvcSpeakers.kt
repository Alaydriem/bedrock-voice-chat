package com.alaydriem.bedrockvoicechat.svc

import de.maxhenkel.voicechat.api.ServerPlayer
import java.util.UUID
import java.util.function.Predicate

/**
 * Who should not be injected into.
 *
 * Populations are disjoint by design — Bedrock players cannot run SVC — but a Java
 * player can run both SVC and the BVC desktop client, and would then hear every
 * remote speaker twice.
 *
 * Decided locally, from state the mod already has. Nothing crosses the wire for it:
 * the sending side does not know who runs what here.
 */
class SvcSpeakers(private val hasLiveBvcClient: (UUID) -> Boolean) {

    fun filter(): Predicate<ServerPlayer> = Predicate { player ->
        !hasLiveBvcClient(player.uuid)
    }
}
