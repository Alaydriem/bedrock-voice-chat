package com.alaydriem.bedrockvoicechat.native

/**
 * The four native calls chat needs from an embedded server.
 *
 * Narrow on purpose. Chat has to survive a server that is not ready yet, and that behaviour is
 * only testable if the calls can be answered without a live handle.
 */
interface ChatFfi {
    /** Registers the mod as this world's chat channel. False while the server is still starting. */
    fun chatRegister(helloJson: String): Boolean

    /** Reports a line a player typed, or something the server said. */
    fun chatReport(chatJson: String): Boolean

    /** Takes every `say` frame waiting to be broadcast, as a JSON array. */
    fun chatDrain(): String?

    /** Releases every chat room this mod registered. */
    fun chatUnregister(): Boolean
}
