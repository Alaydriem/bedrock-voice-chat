package com.alaydriem.bedrockvoicechat.svc

import java.util.concurrent.ConcurrentHashMap

/**
 * Refuses frames Simple Voice Chat cannot play.
 *
 * SVC is 48 kHz only, and everything BVC produces already is: client capture is
 * resampled to 48 kHz before encoding, and jukebox playback is hardcoded to it.
 * `SUPPORTED_SAMPLE_RATES` governs which capture devices are acceptable, not what
 * is broadcast.
 *
 * The rate is still a field on the wire and the peer wire has third-party
 * implementers, so a frame at another rate is representable. It is dropped rather
 * than injected, because playing it raw is fast and wrong-pitched — which an
 * operator hears as a broken bridge rather than as a sender that got the rate wrong.
 *
 * There is deliberately no resampler here. Adding one would put a decode and
 * re-encode on a path no shipped producer reaches, to rescue frames a conforming
 * sender does not send.
 */
class SampleRateGuard {

    private val reported = ConcurrentHashMap.newKeySet<String>()

    fun playable(sampleRate: Int): Boolean = sampleRate == SVC_SAMPLE_RATE

    /**
     * Whether this speaker's wrong rate still needs reporting. At fifty frames a
     * second the diagnosis and a log flood are otherwise the same event.
     */
    fun shouldReport(speaker: String, sampleRate: Int): Boolean =
        !playable(sampleRate) && reported.add(speaker)

    fun forget(speaker: String) {
        reported.remove(speaker)
    }

    companion object {
        private const val SVC_SAMPLE_RATE: Int = 48000
    }
}
