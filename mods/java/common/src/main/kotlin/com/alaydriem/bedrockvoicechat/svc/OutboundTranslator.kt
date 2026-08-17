package com.alaydriem.bedrockvoicechat.svc

import uniffi.bvc_relay_sdk.SdkFrame
import java.util.UUID

/**
 * Turns one SVC microphone packet into one BVC frame.
 *
 * The speaker is supplied by a lookup rather than held, because it must be read at
 * translation time: `PeerIngest::admit` carries the frame's own speaker into the
 * packet it mints, so these coordinates are what spatialise the audio on every
 * receiving client. A cached position is a voice in the wrong place.
 *
 * Keyed on UUID because that is what SVC offers — `ServerPlayer` exposes a uuid and
 * the platform object, and no name. Resolving the name is the platform's job, which
 * is also where Floodgate's gamertag mapping already lives.
 *
 * SVC encodes at 48 kHz mono, which is what BVC expects, so the Opus bytes are
 * copied rather than transcoded.
 */
class OutboundTranslator(
    private val relayWorld: RelayWorld,
    private val speakers: (UUID) -> SpeakerSnapshot?
) {
    fun translate(speaker: UUID, opus: ByteArray, timestampMs: Long): SdkFrame? {
        val snapshot = speakers(speaker) ?: return null

        return SdkFrame(
            speaker = snapshot.name,
            world = relayWorld.id(),
            dimension = snapshot.dimension,
            x = snapshot.x,
            y = snapshot.y,
            z = snapshot.z,
            opus = opus,
            sampleRate = SVC_SAMPLE_RATE,
            timestampMs = timestampMs,
            spatial = true,
            jukebox = null
        )
    }

    companion object {
        // SVC encodes at 48 kHz mono and offers no other rate.
        private const val SVC_SAMPLE_RATE: UInt = 48000u
    }
}
