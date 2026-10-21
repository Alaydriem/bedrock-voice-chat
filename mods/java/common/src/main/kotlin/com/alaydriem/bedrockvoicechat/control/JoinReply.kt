package com.alaydriem.bedrockvoicechat.control

/**
 * What a player reads after `/bvc group join` for a protected group. The join continues in
 * the player's BVC app, where a member compares a code with them; the game never shows it.
 */
object JoinReply {
    /** The reply for a protected join, or null when the join was an ordinary one. */
    fun forResult(result: ControlSendResult, name: String): String? = when (result.outcome) {
        ControlOutcome.FORWARDED -> "[BVC] Your code for $name is in the BVC app."
        ControlOutcome.NO_SESSION -> "[BVC] Open the BVC app to join $name."
        null -> null
    }
}
