package com.alaydriem.bedrockvoicechat.audio

/**
 * Starts and stops server-side audio events.
 *
 * [play] may call back on any thread, or synchronously before it returns. The callback
 * receives the server's JSON response, or null when the request failed.
 */
interface AudioEventPort {
    fun play(json: String, callback: (String?) -> Unit)

    fun stop(eventId: String)
}
