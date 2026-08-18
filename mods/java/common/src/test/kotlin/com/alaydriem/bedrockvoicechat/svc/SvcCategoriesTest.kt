package com.alaydriem.bedrockvoicechat.svc

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNotSame
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Test

class SvcCategoriesTest {

    private fun channels(factory: FakeChannelFactory) =
        SvcChannels(factory, SvcSpeakers { false })

    // So an SVC player can turn music down without turning people down.
    @Test
    fun `a jukebox frame lands in the jukebox category`() {
        val factory = FakeChannelFactory()

        val channel = channels(factory).channelFor(TestFrames.jukebox("jukebox-1", "event-7"))

        assertEquals(SvcCategories.JUKEBOX_ID, channel!!.category)
    }

    // Speech is SVC's ordinary proximity voice and must not be re-categorised, or
    // turning music down would turn people down too.
    @Test
    fun `a speech frame has no category`() {
        val factory = FakeChannelFactory()

        assertNull(channels(factory).channelFor(TestFrames.speech("Steve"))!!.category)
    }

    // Concurrent playbacks carry distinct ids, which is what keeps them on separate
    // channels instead of interleaving into one.
    @Test
    fun `two playbacks from one synthetic speaker get two channels`() {
        val factory = FakeChannelFactory()
        val channels = channels(factory)

        val first = channels.channelFor(TestFrames.jukebox("jukebox-1", "event-7"))
        val second = channels.channelFor(TestFrames.jukebox("jukebox-1", "event-8"))

        assertEquals(2, factory.locationalChannels)
        assertNotSame(first, second)
    }

    // A playback is a block, not a body, so it must never attach to an entity even
    // when a player happens to share the synthetic speaker's name.
    @Test
    fun `a playback never takes an entity channel`() {
        val factory = FakeChannelFactory(livePlayers = TestBodies.living("jukebox-1"))

        channels(factory).channelFor(TestFrames.jukebox("jukebox-1", "event-7"))

        assertEquals(0, factory.entityChannels)
        assertEquals(1, factory.locationalChannels)
    }
}
