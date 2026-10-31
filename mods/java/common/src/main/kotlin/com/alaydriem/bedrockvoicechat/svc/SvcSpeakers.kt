package com.alaydriem.bedrockvoicechat.svc

import com.alaydriem.bedrockvoicechat.svc.link.SvcGroupType
import com.alaydriem.bedrockvoicechat.svc.link.SvcGroupView
import de.maxhenkel.voicechat.api.ServerPlayer
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap
import java.util.function.Predicate

/**
 * Who should not be injected into.
 *
 * Populations are disjoint by design — Bedrock players cannot run SVC — but a Java
 * player can run both SVC and the BVC desktop client, and would then hear every
 * remote speaker twice.
 *
 * The proximity copy of a remote speaker also skips two more listeners: a member of the
 * speaker's linked group, who hears them on a group channel already, and a member of an
 * ISOLATED group, whom SVC keeps from hearing anyone outside it.
 *
 * Decided locally, from state the mod already has. Nothing crosses the wire for it:
 * the sending side does not know who runs what here.
 */
class SvcSpeakers(
    private val groupOf: (UUID) -> SvcGroupView? = { null },
    private val hasLiveBvcClient: (UUID) -> Boolean,
) {
    // Speaker key -> the SVC group their latest frame was linked to.
    private val speakerGroups = ConcurrentHashMap<String, UUID>()

    fun note(speaker: String, svcGroup: UUID?) {
        if (svcGroup == null) speakerGroups.remove(speaker) else speakerGroups[speaker] = svcGroup
    }

    fun hearsThroughBvc(player: UUID): Boolean = hasLiveBvcClient(player)

    /** Re-evaluated per delivery, so a change of client or group needs no new channel. */
    fun filter(speaker: String? = null): Predicate<ServerPlayer> = Predicate { player ->
        if (hasLiveBvcClient(player.uuid)) return@Predicate false
        val listenerGroup = groupOf(player.uuid) ?: return@Predicate true
        if (speaker != null && speakerGroups[speaker] == listenerGroup.id) return@Predicate false
        listenerGroup.type != SvcGroupType.ISOLATED
    }
}
