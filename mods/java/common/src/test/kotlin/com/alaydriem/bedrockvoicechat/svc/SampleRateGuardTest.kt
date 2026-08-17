package com.alaydriem.bedrockvoicechat.svc

import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

class SampleRateGuardTest {

    // Everything BVC produces is already 48 kHz: client capture is resampled before
    // encoding, and jukebox playback is hardcoded to it.
    @Test
    fun `a 48 kHz frame is playable`() {
        assertTrue(SampleRateGuard().playable(48000))
    }

    // Representable on the wire even though nothing we ship emits it, because the
    // peer wire has third-party implementers. Injecting it raw would play fast and
    // at the wrong pitch, which sounds like a broken bridge rather than like a
    // sender that got the rate wrong.
    @Test
    fun `any other rate is refused`() {
        val guard = SampleRateGuard()

        assertFalse(guard.playable(44100))
        assertFalse(guard.playable(16000))
        assertFalse(guard.playable(0))
    }

    // At fifty frames a second per speaker, the honest diagnosis and a log flood
    // are the same event.
    @Test
    fun `a refused speaker is reported once rather than per frame`() {
        val guard = SampleRateGuard()

        assertTrue(guard.shouldReport("Steve", 44100))
        assertFalse(guard.shouldReport("Steve", 44100))
        assertTrue(guard.shouldReport("Alex", 44100))
    }

    @Test
    fun `a playable rate is never reported`() {
        assertFalse(SampleRateGuard().shouldReport("Steve", 48000))
    }

    // A speaker who reconnects at a corrected rate should be reportable again if
    // they later regress, rather than permanently silent in the log.
    @Test
    fun `forgetting a speaker allows one more report`() {
        val guard = SampleRateGuard()

        guard.shouldReport("Steve", 44100)
        guard.forget("Steve")

        assertTrue(guard.shouldReport("Steve", 44100))
    }
}
