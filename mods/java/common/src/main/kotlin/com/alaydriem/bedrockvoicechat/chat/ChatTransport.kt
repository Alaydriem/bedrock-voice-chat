package com.alaydriem.bedrockvoicechat.chat

/**
 * How the mod exchanges chat with the BVC server.
 *
 * Two implementations, chosen by mode exactly as [com.alaydriem.bedrockvoicechat.control.ControlSender]
 * and [com.alaydriem.bedrockvoicechat.audio.AudioEventSender] already do:
 *
 *  - external — [ChatChannel], a WebSocket to a server on the network
 *  - embedded — [FfiChatTransport], direct calls into the server sharing this process
 *
 * Both speak the same `ChatFrame` shapes, so everything above this seam is identical.
 */
interface ChatTransport {
    /** Registers this world and begins exchanging chat. */
    fun start()

    /** Reports a line a player typed in game. */
    fun report(author: String, text: String)

    /**
     * Reports something the server said: a death, a join, a leave, a broadcast.
     *
     * Carries no author, so it renders as a system line. The no-net Bedrock path gets these
     * free because the proxy sees every message the realm sends; a mod has to report them.
     */
    fun event(text: String)

    /** Releases the registration. */
    fun stop()
}
