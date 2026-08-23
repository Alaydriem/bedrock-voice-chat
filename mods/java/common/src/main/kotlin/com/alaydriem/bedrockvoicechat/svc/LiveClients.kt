package com.alaydriem.bedrockvoicechat.svc

import java.util.concurrent.atomic.AtomicReference

/**
 * Which players hold a live voice connection to the BVC server.
 *
 * Two sources, because the two deployments answer differently. Embedded asks over
 * FFI, which is a lookup in a live index and cheap enough to do per call. External
 * asks over HTTP, which is not — so it is refreshed on a schedule and read from a
 * snapshot, and the audio path never waits on a network round trip.
 *
 * The bridge uses this to leave those players out of its injection: one running both
 * Simple Voice Chat and the BVC desktop client would otherwise hear every remote
 * speaker twice.
 */
class LiveClients private constructor(
    private val ask: ((String) -> Boolean)?,
    private val fetch: (() -> List<String>?)?
) {
    private val snapshot = AtomicReference<Set<String>>(emptySet())

    fun isLive(identity: String): Boolean {
        ask?.let { return it(identity) }
        return snapshot.get().contains(identity)
    }

    /**
     * Replaces the snapshot, unless the source could not answer.
     *
     * A failed fetch keeps what was already known. Treating "could not ask" as
     * "nobody is connected" would resume double-audio for every dual-stack player
     * whenever the server blinked.
     */
    fun refresh() {
        val fetcher = fetch ?: return
        val fetched = fetcher() ?: return
        snapshot.set(fetched.toSet())
    }

    val isPolled: Boolean
        get() = fetch != null

    companion object {
        /** Embedded: answered per call over FFI. */
        fun direct(ask: (String) -> Boolean): LiveClients = LiveClients(ask, null)

        /** External: answered from the last successful fetch. */
        fun polled(fetch: () -> List<String>?): LiveClients = LiveClients(null, fetch)
    }
}
