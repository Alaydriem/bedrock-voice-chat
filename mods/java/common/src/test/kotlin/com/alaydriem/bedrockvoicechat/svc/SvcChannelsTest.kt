package com.alaydriem.bedrockvoicechat.svc

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNotNull
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Assertions.assertSame
import org.junit.jupiter.api.Test

class SvcChannelsTest {

    // The Geyser case: a Bedrock player's audio went BVC server -> peer -> bridge,
    // but the speaker is a live body on this very server. An entity channel tracks
    // that body, so it stays correct when frames lag or drop, and writes no
    // position per frame.
    @Test
    fun `a speaker who is a live player here gets an entity channel`() {
        val factory = FakeChannelFactory(livePlayers = TestBodies.living("Steve"))
        val channels = SvcChannels(factory, SvcSpeakers { false })

        channels.channelFor(TestFrames.speech("Steve"))

        assertEquals(1, factory.entityChannels)
        assertEquals(0, factory.locationalChannels)
    }

    // SVC's clients key their talk cache on the channel id, and its name tag
    // renderer asks that cache for the entity's own UUID. Opened under any other id
    // the audio still plays and the speaking indicator appears over nobody, which is
    // how a BVC player came to talk under a disconnected mark.
    @Test
    fun `an entity channel is opened under the body's own UUID`() {
        val factory = FakeChannelFactory(livePlayers = TestBodies.living("Steve"))
        val channels = SvcChannels(factory, SvcSpeakers { false })

        val channel = channels.channelFor(TestFrames.speech("Steve"))

        assertEquals(TestBodies.of("Steve"), channel!!.id)
    }

    // A speaker with no body here has no UUID to borrow, and the name-derived id is
    // what makes a reopened channel the same channel to SVC and its clients.
    @Test
    fun `a locational channel keeps its derived id across a reopen`() {
        val factory = FakeChannelFactory()
        val channels = SvcChannels(factory, SvcSpeakers { false })

        val first = channels.channelFor(TestFrames.speech("Alex"))!!.id
        channels.forget("Alex")
        val second = channels.channelFor(TestFrames.speech("Alex"))!!.id

        assertEquals(first, second)
    }

    // Deployment case 1: the speaker is on a different Minecraft server, so there
    // is no body here and the frame's coordinates are all there is.
    @Test
    fun `a speaker who is not here gets a locational channel`() {
        val factory = FakeChannelFactory()
        val channels = SvcChannels(factory, SvcSpeakers { false })

        channels.channelFor(TestFrames.speech("Alex"))

        assertEquals(1, factory.locationalChannels)
        assertEquals(0, factory.entityChannels)
    }

    // One channel per speaker. A channel per frame would open fifty a second per
    // speaker and give SVC no continuous stream to play.
    @Test
    fun `the same speaker reuses one channel`() {
        val factory = FakeChannelFactory()
        val channels = SvcChannels(factory, SvcSpeakers { false })

        val first = channels.channelFor(TestFrames.speech("Alex"))
        val second = channels.channelFor(TestFrames.speech("Alex"))

        assertSame(first, second)
        assertEquals(1, factory.locationalChannels)
    }

    @Test
    fun `two speakers get two channels`() {
        val factory = FakeChannelFactory()
        val channels = SvcChannels(factory, SvcSpeakers { false })

        channels.channelFor(TestFrames.speech("Alex"))
        channels.channelFor(TestFrames.speech("Steve"))

        assertEquals(2, factory.locationalChannels)
    }

    // A dimension this server does not have is not a channel at some default
    // location. Placing it in the overworld would make a speaker audible to people
    // who are nowhere near them.
    // A dimension this server does not have is not a channel at some default
    // location. Placing it in the overworld would make a speaker audible to people
    // who are nowhere near them.
    @Test
    fun `an unknown dimension yields no channel`() {
        val factory = FakeChannelFactory(knownDimensions = mutableSetOf("overworld"))
        val channels = SvcChannels(factory, SvcSpeakers { false })

        assertNull(channels.channelFor(TestFrames.speech("Alex", dimension = "someones_dim")))
    }

    // A failed open must not be cached as a null channel: the level may exist by
    // the next frame — a dimension loads late, a world is created — and a poisoned
    // entry would keep that speaker silent for the rest of the run.
    @Test
    fun `a failed open is retried rather than cached`() {
        val factory = FakeChannelFactory(knownDimensions = mutableSetOf("overworld"))
        val channels = SvcChannels(factory, SvcSpeakers { false })

        assertNull(channels.channelFor(TestFrames.speech("Alex", dimension = "later")))

        factory.knownDimensions.add("later")

        assertNotNull(channels.channelFor(TestFrames.speech("Alex", dimension = "later")))
        assertEquals(1, factory.locationalChannels)
    }
}
