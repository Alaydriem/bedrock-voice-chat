package com.alaydriem.bedrockvoicechat.svc

import org.junit.jupiter.api.Assertions.assertArrayEquals
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.io.TempDir
import uniffi.bvc_relay_sdk.SdkGroupScope
import java.io.File
import java.util.UUID

class OutboundTranslatorTest {

    private val opus = byteArrayOf(9, 8, 7)
    private val speaker = UUID.randomUUID()

    @Test
    fun `a packet becomes a frame carrying the live position and dimension`(@TempDir dir: File) {
        val relayWorld = RelayWorld(dir)
        val translator = OutboundTranslator(relayWorld) {
            SpeakerSnapshot("Steve", 10.0f, 64.0f, -3.0f, "nether", false)
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
            SpeakerSnapshot("Steve", x, 64.0f, 0.0f, "overworld", false)
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
            SpeakerSnapshot("Steve", 0.0f, 0.0f, 0.0f, "overworld", false)
        }

        assertEquals(relayWorld.id(), translator.translate(speaker, opus, 1L)!!.world)
    }

    // A Java speaker in a linked NORMAL group: without the scope the server routes the speech to
    // whoever stands nearby.
    @Test
    fun `group_scope_reaches_the_frame`(@TempDir dir: File) {
        val translator = OutboundTranslator(RelayWorld(dir)) {
            SpeakerSnapshot("Steve", 0.0f, 64.0f, 0.0f, "overworld", false)
        }
        val scope = SdkGroupScope("ABCD", true)

        assertEquals(scope, translator.translate(speaker, opus, 1L, scope)!!.group)
        assertNull(translator.translate(speaker, opus, 1L)!!.group)
    }

    // The receiving server publishes the frame's speaker over the mod's position post, so a
    // dead SVC player whose frame drops the flag becomes audible to the living.
    @Test
    fun `a dead speaker's frame is marked spectator`(@TempDir dir: File) {
        var dead = true
        val translator = OutboundTranslator(RelayWorld(dir)) {
            SpeakerSnapshot("Steve", 0.0f, 64.0f, 0.0f, "overworld", dead)
        }

        assertTrue(translator.translate(speaker, opus, 1L)!!.spectator)
        dead = false
        assertFalse(translator.translate(speaker, opus, 2L)!!.spectator)
    }
}
