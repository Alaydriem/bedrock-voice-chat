package com.alaydriem.bedrockvoicechat.relay

import kotlinx.coroutines.async
import kotlinx.coroutines.delay
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import kotlinx.coroutines.withTimeoutOrNull
import org.junit.jupiter.api.AfterEach
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNotNull
import org.junit.jupiter.api.Assertions.assertNotEquals
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import uniffi.bvc_relay_sdk.BvcPeer
import uniffi.bvc_relay_sdk.SdkConfig
import java.io.BufferedReader
import java.nio.file.Files

// The gate the design rests on: that the generated bindings load, carry a frame
// off a real wire, and release a coroutine parked in `nextFrame` when the
// session is shut down. uniffi supports no cancellation, so the last of those
// has no fallback — a plugin that cannot do it hangs on disable.
class SdkSmokeTest {
    private var echo: Process? = null

    @AfterEach
    fun stopEchoPeer() {
        echo?.destroyForcibly()
    }

    // `burst` bounds how many frames the peer sends before going quiet. A test
    // asserting that a parked read is released cannot use a peer that never
    // stops talking; it would be asserting which of the two won a race.
    private fun startEchoPeer(burst: Int? = null, jukebox: String? = null): String {
        val binary = System.getProperty("bvc.echoPeer")
            ?: error("bvc.echoPeer system property is not set")

        val command = mutableListOf(binary)
        if (burst != null) {
            command += listOf("--burst", burst.toString())
        }
        if (jukebox != null) {
            command += listOf("--jukebox", jukebox)
        }

        val process = ProcessBuilder(command).redirectErrorStream(false).start()
        echo = process

        val reader = BufferedReader(process.inputStream.reader())
        return generateSequence { reader.readLine() }
            .firstOrNull { it.startsWith("PEERLINK=") }
            ?.removePrefix("PEERLINK=")
            ?: error("the echo peer did not print a peer link")
    }

    private fun config(peerlink: String) = SdkConfig(
        nodeDir = Files.createTempDirectory("bvc-sdk-test").toString(),
        peerlink = peerlink,
        worlds = listOf("W1"),
        relayUrl = null,
        inboxCapacity = 8u,
    )

    // An UnsatisfiedLinkError here means the native library is not resolvable, or
    // JNA was not relocated. Neither is fixable in Kotlin.
    @Test
    fun `a frame arrives over the generated bindings`() = runBlocking {
        val peerlink = startEchoPeer(burst = null)
        val peer = BvcPeer.open(config(peerlink))

        val frame = withTimeout(30_000) { peer.nextFrame() }

        assertNotNull(frame, "a frame must arrive from the echo peer")
        assertEquals("EchoPeer", frame!!.speaker)
        assertEquals("W1", frame.world)
        assertEquals(1.0f, frame.x)
        assertTrue(frame.opus.isNotEmpty())
        assertNull(frame.jukebox, "speech must not be reported as a playback")

        peer.shutdown()
    }

    // The field exists so a bridge never has to match a prefix on the speaker's
    // name to know what it is being handed.
    @Test
    fun `a jukebox id reaches the bindings`() = runBlocking {
        val peerlink = startEchoPeer(jukebox = "evt-9")
        val peer = BvcPeer.open(config(peerlink))

        val frame = withTimeout(30_000) { peer.nextFrame() }

        assertNotNull(frame)
        assertEquals("evt-9", frame!!.jukebox)

        peer.shutdown()
    }

    // The constraint the entire surface is shaped around. Cancelling the
    // coroutine would leave it parked forever; shutdown is the only thing that
    // can end it.
    @Test
    fun `shutdown releases a coroutine parked in nextFrame`() = runBlocking {
        val peerlink = startEchoPeer(burst = 2)
        val peer = BvcPeer.open(config(peerlink))

        // Drain until the peer goes quiet, so the read below is genuinely parked
        // rather than about to be handed a queued frame.
        while (withTimeoutOrNull(5_000) { peer.nextFrame() } != null) {
            // draining
        }

        val parked = async { peer.nextFrame() }
        delay(500)

        peer.shutdown()

        val result = withTimeout(15_000) { parked.await() }
        assertNull(result, "shutdown must make a parked nextFrame return null")
    }

    // A bridge prints its own link for the operator to paste into config.hcl, so
    // it must report the identity it is using rather than the one it was given.
    @Test
    fun `the sdk reports its own peer link`() = runBlocking {
        val peerlink = startEchoPeer(burst = 1)
        val peer = BvcPeer.open(config(peerlink))

        val own = peer.peerlink()
        assertTrue(own.startsWith("bvcpeer"), "not a peer link: $own")
        assertNotEquals(peerlink, own, "reported the peer's link, not its own")

        peer.shutdown()
    }
}
