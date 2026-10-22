package com.alaydriem.bedrockvoicechat.control

/**
 * What became of a join for a protected group, which the game cannot join itself: the server
 * handed it to the player's BVC app, or the player had no app session to take it.
 */
enum class ControlOutcome {
    FORWARDED,
    NO_SESSION;

    companion object {
        /** `POST /api/control`: 202 forwarded, 409 no session. */
        fun fromHttpStatus(status: Int): ControlOutcome? = when (status) {
            202 -> FORWARDED
            409 -> NO_SESSION
            else -> null
        }

        /** `bvc_client_action`: 1 forwarded, -2 no session. */
        fun fromFfiCode(code: Int): ControlOutcome? = when (code) {
            1 -> FORWARDED
            -2 -> NO_SESSION
            else -> null
        }
    }
}
