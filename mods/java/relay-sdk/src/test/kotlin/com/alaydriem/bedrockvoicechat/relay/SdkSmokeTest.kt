package com.alaydriem.bedrockvoicechat.relay

import kotlinx.coroutines.async
import kotlinx.coroutines.delay
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import kotlinx.coroutines.withTimeoutOrNull
import org.junit.jupiter.api.AfterEach
import org.junit.jupiter.api.Assertions.assertArrayEquals
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertNotNull
import org.junit.jupiter.api.Assertions.assertNotEquals
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Assertions.assertThrows
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import uniffi.bvc_relay_sdk.BvcPeer
import uniffi.bvc_relay_sdk.SdkConfig
import uniffi.bvc_relay_sdk.SdkException
import uniffi.bvc_relay_sdk.SdkFrame
import java.io.BufferedReader
import java.nio.file.Files

// The gate the design rests on: that the generated bindings load, carry a frame
// off a real wire, and release a coroutine parked in `nextFrame` when the
// session is shut down. uniffi supports no cancellation, so the last of those
// has no fallback — a plugin that cannot do it hangs on disable.
class SdkSmokeTest {
    private var echoPeer: Process? = null

    @AfterEach
    fun stopEchoPeer() {
        echoPeer?.destroyForcibly()
    }

    // `burst` bounds how many frames the peer sends before going quiet. A test
    // asserting that a parked read is released cannot use a peer that never
    // stops talking; it would be asserting which of the two won a race.
    //
    // `echo` reflects back whatever the peer is sent. Paired with `burst = 0` it
    // makes the peer silent except for what it receives, so a frame arriving at
    // the caller is unambiguously the one the caller just sent.
    private fun startEchoPeer(
        burst: Int? = null,
        jukebox: String? = null,
        echo: Boolean = false,
    ): String {
        val binary = System.getProperty("bvc.echoPeer")
            ?: error("bvc.echoPeer system property is not set")

        val command = mutableListOf(binary)
        if (burst != null) {
            command += listOf("--burst", burst.toString())
        }
        if (jukebox != null) {
            command += listOf("--jukebox", jukebox)
        }
        if (echo) {
            command += "--echo"
        }

        val process = ProcessBuilder(command).redirectErrorStream(false).start()
        echoPeer = process

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

    // `open` returns before the dial completes, so a send issued straight after it
    // would be refused for the right reason at the wrong moment.
    private suspend fun awaitConnected(peer: BvcPeer, timeoutMs: Long) {
        withTimeout(timeoutMs) {
            while (!peer.isConnected()) {
                delay(100)
            }
        }
    }

    private fun outboundFrame(jukebox: String?) = SdkFrame(
        speaker = "BridgeSpeaker",
        world = "W1",
        x = 4.0f,
        y = 64.0f,
        z = -2.0f,
        opus = byteArrayOf(7, 7, 7),
        sampleRate = 48000u,
        timestampMs = 1234L,
        spatial = true,
        jukebox = jukebox,
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

    // The direction a bridge exists for. The peer is silent except for what it is
    // sent, so a frame arriving here is the one that just went out — proving the
    // send crossed the FFI boundary, reached the wire, and came back.
    @Test
    fun `a sent frame reaches the peer and returns`() = runBlocking {
        val peerlink = startEchoPeer(burst = 0, echo = true)
        val peer = BvcPeer.open(config(peerlink))
        awaitConnected(peer, 30_000)

        peer.send(outboundFrame(jukebox = null))

        val echoed = withTimeout(30_000) { peer.nextFrame() }

        assertNotNull(echoed, "the peer never returned the frame that was sent")
        assertEquals("BridgeSpeaker", echoed!!.speaker)
        assertEquals("W1", echoed.world)
        assertEquals(4.0f, echoed.x)
        assertEquals(1234L, echoed.timestampMs)
        assertArrayEquals(byteArrayOf(7, 7, 7), echoed.opus)

        peer.shutdown()
    }

    // Outbound jukebox audio is a real case: a bridge carrying a playback into BVC
    // must be able to say so, not only recognise one arriving.
    @Test
    fun `a sent jukebox id survives the round trip`() = runBlocking {
        val peerlink = startEchoPeer(burst = 0, echo = true)
        val peer = BvcPeer.open(config(peerlink))
        awaitConnected(peer, 30_000)

        peer.send(outboundFrame(jukebox = "evt-77"))

        val echoed = withTimeout(30_000) { peer.nextFrame() }

        assertNotNull(echoed)
        assertEquals("evt-77", echoed!!.jukebox)

        peer.shutdown()
    }

    // Refused rather than queued, and the refusal has to reach Kotlin as an
    // exception rather than being swallowed into a silent success.
    //
    // The peer is started only to mint a well-formed link and is then killed, so
    // the key is valid and nothing answers it.
    @Test
    fun `sending without a link throws rather than queueing`() = runBlocking {
        val peerlink = startEchoPeer(burst = 0)
        echoPeer?.destroyForcibly()
        echoPeer?.waitFor()

        val peer = BvcPeer.open(config(peerlink))

        assertFalse(peer.isConnected())
        assertThrows(SdkException::class.java) {
            peer.send(outboundFrame(jukebox = null))
        }

        peer.shutdown()
    }
}
