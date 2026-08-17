package com.alaydriem.bedrockvoicechat.svc

import org.slf4j.LoggerFactory
import uniffi.bvc_relay_sdk.SdkFrame

/**
 * Puts one BVC frame into Simple Voice Chat.
 *
 * `AudioChannel.send` takes raw Opus and both sides run at 48 kHz, so this is a copy
 * rather than a transcode. `AudioPlayer` is for callers holding PCM and is not used.
 */
class InboundTranslator(
    private val channels: SvcChannels,
    private val guard: SampleRateGuard
) {
    fun inject(frame: SdkFrame) {
        val rate = frame.sampleRate.toInt()

        if (!guard.playable(rate)) {
            if (guard.shouldReport(frame.speaker, rate)) {
                logger.warn(
                    "Dropping audio from {}: {} Hz, and Simple Voice Chat plays 48000 Hz only",
                    frame.speaker,
                    rate
                )
            }
            return
        }

        channels.channelFor(frame)?.send(frame.opus)
    }

    companion object {
        private val logger = LoggerFactory.getLogger("BVC SVC")
    }
}
