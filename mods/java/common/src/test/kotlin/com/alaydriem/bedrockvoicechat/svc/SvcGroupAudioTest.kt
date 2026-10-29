package com.alaydriem.bedrockvoicechat.svc

import com.alaydriem.bedrockvoicechat.svc.link.LinkEntry
import com.alaydriem.bedrockvoicechat.svc.link.SvcLinkTable
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import uniffi.bvc_relay_sdk.SdkGroupScope
import java.util.UUID

class SvcGroupAudioTest {
    private val group = UUID.randomUUID()
    private val steve = UUID.randomUUID()
    private val alex = UUID.randomUUID()
    private val bob = UUID.randomUUID()
    private val table = SvcLinkTable()
    private val opened = HashMap<Pair<UUID, String>, FakeAudioChannel>()
    private val audio = SvcGroupAudio(
        table = table,
        speakers = SvcSpeakers { it == bob },
        membersOf = { if (it == group) listOf(steve, alex, bob) else emptyList() },
        staticChannelFor = { listener, key ->
            FakeAudioChannel(UUID.randomUUID()).also { opened[listener to key] = it }
        },
        speakerOf = { if (it == "Alex") alex else null },
    )

    private fun scope(membersOnly: Boolean) = SdkGroupScope("ABCD", membersOnly)

    private fun link(membersOnly: Boolean) =
        table.put(LinkEntry(group, "ABCD", membersOnly, LinkEntry.Origin.SVC))

    // Group members hear each other at any distance, so each Java member gets the frame on a
    // non-spatial channel of their own. The speaker never hears themselves, and a member with the
    // BVC client hears through BVC.
    @Test
    fun `members_only_frame_reaches_members_only`() {
        link(membersOnly = true)

        val proximityToo = audio.deliver(TestFrames.speech("Alex", group = scope(true)))

        assertFalse(proximityToo)
        assertEquals(setOf(steve to "Alex"), opened.keys)
        assertEquals(1, opened.getValue(steve to "Alex").sent.size)
    }

    @Test
    fun `open_group_frame_reaches_members_and_proximity`() {
        link(membersOnly = false)

        val proximityToo = audio.deliver(TestFrames.speech("Alex", group = scope(false)))

        assertTrue(proximityToo)
        assertTrue((steve to "Alex") in opened)
    }

    @Test
    fun `unlinked_open_scope_uses_proximity`() {
        val proximityToo = audio.deliver(TestFrames.speech("Alex", group = scope(false)))

        assertTrue(proximityToo)
        assertTrue(opened.isEmpty())
    }

    // The table can lag the server: at bridge start, or while a link is being remade. The
    // server's members-only marker alone keeps the speech away from nearby players.
    @Test
    fun `unlinked_members_only_scope_is_dropped`() {
        val proximityToo = audio.deliver(TestFrames.speech("Alex", group = scope(true)))

        assertFalse(proximityToo)
        assertTrue(opened.isEmpty())
    }

    // The marker can lag the link table: a members-only marker on a channel the bridge holds as
    // OPEN still keeps the speech out of proximity.
    @Test
    fun `either side saying members only keeps proximity off`() {
        link(membersOnly = false)

        assertFalse(audio.deliver(TestFrames.speech("Alex", group = scope(true))))
    }

    @Test
    fun `a member keeps one channel per speaker`() {
        link(membersOnly = true)
        var opens = 0
        val counting = SvcGroupAudio(
            table = table,
            speakers = SvcSpeakers { false },
            membersOf = { listOf(steve) },
            staticChannelFor = { _, _ -> opens++; FakeAudioChannel(UUID.randomUUID()) },
            speakerOf = { null },
        )

        counting.deliver(TestFrames.speech("Alex", group = scope(true)))
        counting.deliver(TestFrames.speech("Alex", group = scope(true)))

        assertEquals(1, opens)
    }
}
