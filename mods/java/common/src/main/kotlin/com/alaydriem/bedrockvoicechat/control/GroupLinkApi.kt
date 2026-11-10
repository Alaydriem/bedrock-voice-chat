package com.alaydriem.bedrockvoicechat.control

/**
 * The BVC side of the Simple Voice Chat bridge's group links. Every call blocks, so callers run
 * it on the link thread, never the server thread.
 *
 * [player] arguments are in-game names; the server resolves them as it does any actor.
 */
interface GroupLinkApi {
    /** The new channel id, or null when the server refused or could not be reached. */
    fun createLinked(creator: String, name: String, membersOnly: Boolean): String?
    fun close(channel: String): Boolean
    fun lease(channels: List<String>): Boolean
    /** False when BVC refused the join or could not be reached. */
    fun join(player: String, channel: String): Boolean
    fun leave(player: String): Boolean
    fun drain(after: Long): GroupLinkDrainJson?
    fun channels(): List<LinkedChannelJson>?
}
