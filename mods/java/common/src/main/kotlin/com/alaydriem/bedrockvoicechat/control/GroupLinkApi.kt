package com.alaydriem.bedrockvoicechat.control

import java.util.UUID

/**
 * The BVC side of the Simple Voice Chat bridge's group links. Every call blocks, so callers run
 * it on the link thread, never the server thread.
 *
 * [player] arguments are in-game names; the server resolves them as it does any actor. [uuid] is
 * that player's platform UUID when they are online, which the server uses to decide whether an
 * alias on the name is theirs.
 */
interface GroupLinkApi {
    /** The new channel id, or null when the server refused or could not be reached. */
    fun createLinked(creator: String, name: String, membersOnly: Boolean): String?
    fun close(channel: String): Boolean
    fun lease(channels: List<String>): Boolean
    /** False when BVC refused the join or could not be reached. */
    fun join(player: String, uuid: UUID?, channel: String): Boolean
    fun leave(player: String, uuid: UUID?): Boolean
    fun drain(after: Long): GroupLinkDrainJson?
    fun channels(): List<LinkedChannelJson>?
}
