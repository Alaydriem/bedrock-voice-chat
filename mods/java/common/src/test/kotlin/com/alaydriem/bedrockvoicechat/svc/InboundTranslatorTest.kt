package com.alaydriem.bedrockvoicechat.svc

import org.junit.jupiter.api.Assertions.assertArrayEquals
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

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
}
