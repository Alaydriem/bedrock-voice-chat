package com.alaydriem.bedrockvoicechat.svc

import org.junit.jupiter.api.Assertions.assertArrayEquals
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.io.TempDir
import java.io.File
import java.util.UUID

class OutboundTranslatorTest {

    private val opus = byteArrayOf(9, 8, 7)
    private val speaker = UUID.randomUUID()

    @Test
    fun `a packet becomes a frame carrying the live position and dimension`(@TempDir dir: File) {
        val relayWorld = RelayWorld(dir)
        val translator = OutboundTranslator(relayWorld) {
            SpeakerSnapshot("Steve", 10.0f, 64.0f, -3.0f, "nether")
        }

        val frame = translator.translate(speaker, opus, 1234L)!!

        assertEquals("Steve", frame.speaker)
        assertEquals(relayWorld.id(), frame.world)
        assertEquals("nether", frame.dimension)
        assertEquals(10.0f, frame.x)
        assertEquals(64.0f, frame.y)
        assertEquals(-3.0f, frame.z)
        assertArrayEquals(opus, frame.opus)
        assertEquals(48000u, frame.sampleRate)
        assertEquals(1234L, frame.timestampMs)
        assertNull(frame.jukebox)
    }

    // The position is read at translation time, not cached. `admit` carries the
    // frame's own speaker into the packet it mints, so a stale position is a voice
    // in the wrong place on every receiving client.
    @Test
    fun `each packet reads the position again`(@TempDir dir: File) {
        var x = 1.0f
        val translator = OutboundTranslator(RelayWorld(dir)) {
            SpeakerSnapshot("Steve", x, 64.0f, 0.0f, "overworld")
        }

        val first = translator.translate(speaker, opus, 1L)!!
        x = 99.0f
        val second = translator.translate(speaker, opus, 2L)!!

        assertEquals(1.0f, first.x)
        assertEquals(99.0f, second.x)
    }

    // Dropped here rather than sent with a guessed position and refused at the far
    // boundary, where the reason would be a warning on someone else's server.
    @Test
    fun `a speaker who cannot be located produces no frame`(@TempDir dir: File) {
        val translator = OutboundTranslator(RelayWorld(dir)) { null }

        assertNull(translator.translate(speaker, opus, 1L))
    }

    // Every frame declares the world this server peers as, or the far side refuses
    // it as NoWorld.
    @Test
    fun `every frame carries the relay world`(@TempDir dir: File) {
        val relayWorld = RelayWorld(dir)
        val translator = OutboundTranslator(relayWorld) {
            SpeakerSnapshot("Steve", 0.0f, 0.0f, 0.0f, "overworld")
        }

        assertEquals(relayWorld.id(), translator.translate(speaker, opus, 1L)!!.world)
    }
}
