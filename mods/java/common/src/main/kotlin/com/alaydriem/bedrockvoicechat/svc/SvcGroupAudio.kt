package com.alaydriem.bedrockvoicechat.svc

import com.alaydriem.bedrockvoicechat.svc.link.SvcLinkTable
import de.maxhenkel.voicechat.api.audiochannel.AudioChannel
import uniffi.bvc_relay_sdk.SdkFrame
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap

/**
 * Plays a linked group's audio to the group's Java members at any distance.
 *
 * SVC plays entity and locational channels only within range, so group members far apart would
 * not hear each other through them. Each member gets a non-spatial static channel per speaker.
 * The speaker never hears themselves, and a member with the BVC client hears through BVC.
 *
 * [staticChannelFor] opens a listener's channel for a speaker key; [speakerOf] finds a speaker's
 * body on this server, for a Bedrock speaker who is also in the group.
 */
class SvcGroupAudio(
    private val table: SvcLinkTable,
    private val speakers: SvcSpeakers,
    private val membersOf: (UUID) -> List<UUID>,
    private val staticChannelFor: (listener: UUID, key: String) -> AudioChannel?,
    private val speakerOf: (String) -> UUID?,
) {
    private val channels = ConcurrentHashMap<Pair<UUID, String>, AudioChannel>()

    /** Delivers to the group when the frame's group is linked. Returns whether proximity plays it too. */
    fun deliver(frame: SdkFrame): Boolean {
        val scope = frame.group
        val entry = scope?.let { table.byBvc(it.channel) }
        speakers.note(frame.speaker, entry?.svcGroup)
        if (scope == null) return true
        // The table can lag the server, at bridge start or while a link is remade. The server's
        // members-only marker alone keeps the speech away from nearby players.
        if (entry == null) return !scope.membersOnly

        val self = speakerOf(frame.speaker)
        for (member in membersOf(entry.svcGroup)) {
            if (member == self || speakers.hearsThroughBvc(member)) continue
            channelFor(member, frame.speaker)?.send(frame.opus)
        }
        return !(scope.membersOnly || entry.membersOnly)
    }

    private fun channelFor(listener: UUID, speaker: String): AudioChannel? {
        val key = listener to speaker
        channels[key]?.let { existing ->
            if (!existing.isClosed) return existing
            channels.remove(key, existing)
        }
        val opened = staticChannelFor(listener, speaker) ?: return null
        return channels.putIfAbsent(key, opened) ?: opened
    }
}
