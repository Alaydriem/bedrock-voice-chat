package com.alaydriem.bedrockvoicechat.svc

import de.maxhenkel.voicechat.api.ServerPlayer
import de.maxhenkel.voicechat.api.audiochannel.AudioChannel
import de.maxhenkel.voicechat.api.packets.MicrophonePacket
import uniffi.bvc_relay_sdk.SdkFrame
import java.util.UUID
import java.util.function.Predicate

/**
 * Records what a channel was told, so a test can assert on delivery rather than on
 * the calls that produced it.
 */
class FakeAudioChannel(private val id: UUID) : AudioChannel {

    val sent: MutableList<ByteArray> = mutableListOf()
    var flushes: Int = 0

    // Not named `filter`: a Kotlin property of that name generates a `setFilter`
    // that collides with the interface method it is meant to record.
    var appliedFilter: Predicate<ServerPlayer>? = null

    private var category: String? = null
    private var closed: Boolean = false

    override fun send(data: ByteArray) {
        sent.add(data)
    }

    override fun send(packet: MicrophonePacket) {
        sent.add(packet.opusEncodedData)
    }

    override fun setFilter(filter: Predicate<ServerPlayer>?) {
        this.appliedFilter = filter
    }

    override fun flush() {
        flushes += 1
    }

    override fun isClosed(): Boolean = closed

    override fun getId(): UUID = id

    override fun getCategory(): String? = category

    override fun setCategory(category: String?) {
        this.category = category
    }
}

/**
 * A channel factory that opens fakes and counts which kind was asked for.
 *
 * `livePlayers` decides whether a speaker has a body here, which is the whole
 * entity-versus-locational question.
 */
class FakeChannelFactory(
    private val livePlayers: Set<String> = emptySet(),
    val knownDimensions: MutableSet<String> = mutableSetOf("overworld", "nether", "the_end")
) : SvcChannelFactory {

    var entityChannels: Int = 0
    var locationalChannels: Int = 0
    val opened: MutableList<FakeAudioChannel> = mutableListOf()

    override fun entityChannel(id: UUID, speaker: String): AudioChannel? {
        if (!livePlayers.contains(speaker)) {
            return null
        }
        entityChannels += 1
        return FakeAudioChannel(id).also { opened.add(it) }
    }

    override fun locationalChannel(
        id: UUID,
        dimension: String,
        x: Double,
        y: Double,
        z: Double
    ): AudioChannel? {
        if (!knownDimensions.contains(dimension)) {
            return null
        }
        locationalChannels += 1
        return FakeAudioChannel(id).also { opened.add(it) }
    }
}

/** Frames shaped the way the bridge produces them. */
object TestFrames {

    fun speech(
        speaker: String,
        dimension: String = "overworld",
        sampleRate: UInt = 48000u,
        opus: ByteArray = byteArrayOf(1, 2, 3)
    ) = SdkFrame(
        speaker = speaker,
        world = "W1",
        dimension = dimension,
        x = 1.0f,
        y = 64.0f,
        z = 2.0f,
        opus = opus,
        sampleRate = sampleRate,
        timestampMs = 1L,
        spatial = true,
        jukebox = null
    )

    fun jukebox(speaker: String, eventId: String) = speech(speaker).copy(jukebox = eventId)
}
