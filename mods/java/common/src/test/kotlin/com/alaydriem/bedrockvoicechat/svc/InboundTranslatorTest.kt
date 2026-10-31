package com.alaydriem.bedrockvoicechat.svc

import org.junit.jupiter.api.Assertions.assertArrayEquals
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import com.alaydriem.bedrockvoicechat.svc.link.LinkEntry
import com.alaydriem.bedrockvoicechat.svc.link.SvcLinkTable
import uniffi.bvc_relay_sdk.SdkGroupScope
import java.util.UUID

class InboundTranslatorTest {

    private fun translator(factory: FakeChannelFactory) =
        InboundTranslator(SvcChannels(factory, SvcSpeakers { false }), SampleRateGuard())

    // Both sides run at 48 kHz, so the Opus bytes are copied rather than decoded
    // and re-encoded.
    @Test
    fun `opus reaches the channel byte for byte`() {
        val factory = FakeChannelFactory()
        val opus = byteArrayOf(4, 5, 6, 7)

        translator(factory).inject(TestFrames.speech("Alex", opus = opus))

        assertEquals(1, factory.opened.size)
        assertArrayEquals(opus, factory.opened.first().sent.single())
    }

    // Dropped rather than injected: playing it raw is fast and wrong-pitched, which
    // sounds like a broken bridge rather than like a sender that got the rate wrong.
    @Test
    fun `a frame at another rate is dropped rather than played`() {
        val factory = FakeChannelFactory()

        translator(factory).inject(TestFrames.speech("Alex", sampleRate = 44100u))

        assertEquals(0, factory.opened.size)
        assertEquals(0, factory.locationalChannels)
    }

    // The suppression filter is applied when the channel is opened, not per frame,
    // so a listener running the BVC client never receives from it.
    @Test
    fun `an opened channel carries the suppression filter`() {
        val factory = FakeChannelFactory()

        translator(factory).inject(TestFrames.speech("Alex"))

        assertTrue(factory.opened.first().appliedFilter != null)
    }

    @Test
    fun `successive frames from one speaker share a channel`() {
        val factory = FakeChannelFactory()
        val translator = translator(factory)

        translator.inject(TestFrames.speech("Alex", opus = byteArrayOf(1)))
        translator.inject(TestFrames.speech("Alex", opus = byteArrayOf(2)))

        assertEquals(1, factory.opened.size)
        assertEquals(2, factory.opened.first().sent.size)
    }

    private val group = UUID.randomUUID()
    private val steve = UUID.randomUUID()
    private val table = SvcLinkTable()
    private val delivered = ArrayList<UUID>()

    private fun groupTranslator(factory: FakeChannelFactory, membersOnly: Boolean): InboundTranslator {
        table.put(LinkEntry(group, "ABCD", membersOnly, LinkEntry.Origin.SVC))
        val speakers = SvcSpeakers { false }
        val audio = SvcGroupAudio(
            table = table,
            speakers = speakers,
            membersOf = { listOf(steve) },
            staticChannelFor = { listener, _ -> delivered.add(listener); FakeAudioChannel(UUID.randomUUID()) },
            speakerOf = { null },
        )
        return InboundTranslator(SvcChannels(factory, speakers), SampleRateGuard(), audio)
    }

    // A Bedrock speaker in a linked NORMAL or ISOLATED group reaches the group and nobody nearby.
    @Test
    fun `members_only_frame_reaches_members_only`() {
        val factory = FakeChannelFactory()

        groupTranslator(factory, membersOnly = true)
            .inject(TestFrames.speech("Alex", group = SdkGroupScope("ABCD", true)))

        assertEquals(listOf(steve), delivered)
        assertEquals(0, factory.opened.size)
    }

    @Test
    fun `open_group_frame_reaches_members_and_proximity`() {
        val factory = FakeChannelFactory()

        groupTranslator(factory, membersOnly = false)
            .inject(TestFrames.speech("Alex", group = SdkGroupScope("ABCD", false)))

        assertEquals(listOf(steve), delivered)
        assertEquals(1, factory.opened.size)
    }

    // An ISOLATED group's own speaker reaches its Java members on the group channel, which the
    // proximity filter does not touch.
    @Test
    fun `isolated_listener_hears_own_group_speaker`() {
        val factory = FakeChannelFactory()

        groupTranslator(factory, membersOnly = true)
            .inject(TestFrames.speech("Alex", group = SdkGroupScope("ABCD", true)))

        assertTrue(steve in delivered)
    }

    @Test
    fun `unlinked_open_scope_uses_proximity`() {
        val factory = FakeChannelFactory()

        groupTranslator(factory, membersOnly = true)
            .inject(TestFrames.speech("Alex", group = SdkGroupScope("ELSEWHERE", false)))

        assertTrue(delivered.isEmpty())
        assertEquals(1, factory.opened.size)
    }

    // A members-only frame the bridge cannot place in a group reaches nobody, never proximity.
    @Test
    fun `unlinked_members_only_scope_reaches_nobody`() {
        val factory = FakeChannelFactory()

        groupTranslator(factory, membersOnly = true)
            .inject(TestFrames.speech("Alex", group = SdkGroupScope("ELSEWHERE", true)))

        assertTrue(delivered.isEmpty())
        assertEquals(0, factory.opened.size)
    }
}
