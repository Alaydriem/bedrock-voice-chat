package com.alaydriem.bedrockvoicechat.svc

import de.maxhenkel.voicechat.api.audiochannel.AudioChannel
import org.slf4j.LoggerFactory
import uniffi.bvc_relay_sdk.SdkFrame
import java.nio.charset.StandardCharsets
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap

/**
 * One SVC audio channel per remote speaker.
 *
 * Entity-backed when the speaker is a live player on this server — the Geyser case,
 * where a Bedrock player's audio reached us over the peer link but their body is
 * here. The channel then tracks the body, which stays correct when frames lag and
 * writes no position per frame.
 *
 * Locational otherwise, because a speaker on another Minecraft server has no body
 * here and the frame's coordinates are all there is.
 */
class SvcChannels(
    private val factory: SvcChannelFactory,
    private val speakers: SvcSpeakers
) {
    private val channels = ConcurrentHashMap<String, AudioChannel>()

    fun channelFor(frame: SdkFrame): AudioChannel? {
        val key = keyFor(frame)

        channels[key]?.let { existing ->
            if (!existing.isClosed) {
                return existing
            }
            channels.remove(key, existing)
        }

        // Opened outside `computeIfAbsent` so a failed open is not cached. The
        // level may exist by the next frame — a dimension loads late, a world is
        // created — and a poisoned null would keep that speaker silent for the
        // rest of the run.
        val opened = open(frame) ?: return null
        opened.setFilter(speakers.filter())

        // Playback gets its own volume category so a listener can turn music down
        // without turning people down. Speech is left uncategorised, which is SVC's
        // ordinary proximity voice.
        if (frame.jukebox != null) {
            opened.category = SvcCategories.JUKEBOX_ID
        }

        return channels.putIfAbsent(key, opened) ?: opened
    }

    fun forget(speaker: String) {
        channels.remove(speaker)
    }

    /**
     * Playback is keyed on its own id rather than the speaker.
     *
     * Concurrent playbacks carry distinct ids, and collapsing them onto the speaker
     * would interleave two pieces of audio into one channel.
     */
    private fun keyFor(frame: SdkFrame): String = frame.jukebox ?: frame.speaker

    private fun open(frame: SdkFrame): AudioChannel? {
        // A playback is a block, not a body, so it never resolves to an entity.
        if (frame.jukebox == null) {
            factory.entityChannel(frame.speaker)?.let {
                logger.info("Opened an entity channel for {}", frame.speaker)
                return it
            }

            // The two kinds sound different to a listener and the fallback is
            // otherwise silent, so which one a speaker got is stated rather than
            // inferred from how their voice behaves.
            logger.info(
                "No body here for {}; opening a locational channel at {} {} {} in {}",
                frame.speaker,
                frame.x,
                frame.y,
                frame.z,
                frame.dimension
            )
        }

        return factory.locationalChannel(
            channelId(keyFor(frame)),
            frame.dimension,
            frame.x.toDouble(),
            frame.y.toDouble(),
            frame.z.toDouble()
        )
    }

    /**
     * Derived from the key rather than random, so a channel that has to be reopened
     * is the same channel to SVC and to its clients.
     *
     * Locational channels only. A speaker with a body here gets that body's UUID,
     * which is what SVC's name tag renderer looks the channel up by.
     */
    private fun channelId(key: String): UUID =
        UUID.nameUUIDFromBytes(key.toByteArray(StandardCharsets.UTF_8))

    companion object {
        private val logger = LoggerFactory.getLogger("BVC SVC")
    }
}
