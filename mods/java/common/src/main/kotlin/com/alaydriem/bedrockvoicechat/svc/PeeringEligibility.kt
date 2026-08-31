package com.alaydriem.bedrockvoicechat.svc

/**
 * Whether the external BVC server this mod talks to will accept a voice bridge, and where
 * that bridge should dial.
 *
 * `fetch` is injected so the caching behaviour is testable without an HTTP server. The peer
 * link itself is BVC's business; this type only decides how often to ask for it.
 */
class PeeringEligibility(private val fetch: () -> String?) {

    // Held only on success. A server that was briefly unreachable would otherwise be
    // remembered as one that does not peer, and the operator would have to restart the game
    // server to pair.
    private var cached: String? = null

    fun resolve(): String? {
        cached?.let { return it }

        return fetch().also { cached = it }
    }

    fun isEligible(): Boolean = resolve() != null
}
