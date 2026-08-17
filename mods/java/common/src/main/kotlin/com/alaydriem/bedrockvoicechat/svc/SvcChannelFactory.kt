package com.alaydriem.bedrockvoicechat.svc

import de.maxhenkel.voicechat.api.audiochannel.AudioChannel
import java.util.UUID

/**
 * Opens SVC audio channels.
 *
 * A seam rather than a direct `VoicechatServerApi` dependency: that interface
 * declares dozens of methods across groups, listeners, senders and config, and
 * channel selection needs two of them. Narrowing it here keeps the decision
 * testable without a fake that has to answer for the whole API — and the parts of
 * it that are platform-shaped, the entity and level lookups, are supplied by the
 * platform behind this same seam.
 */
interface SvcChannelFactory {

    /**
     * A channel tracking a live body on this server, or null when the speaker is
     * not a player here.
     */
    fun entityChannel(id: UUID, speaker: String): AudioChannel?

    /**
     * A channel at a fixed point, for a speaker with no body here. Null when the
     * named dimension has no level on this server.
     */
    fun locationalChannel(
        id: UUID,
        dimension: String,
        x: Double,
        y: Double,
        z: Double
    ): AudioChannel?
}
