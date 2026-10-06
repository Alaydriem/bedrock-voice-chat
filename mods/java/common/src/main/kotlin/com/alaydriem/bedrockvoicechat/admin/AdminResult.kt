package com.alaydriem.bedrockvoicechat.admin

/**
 * The `bvc_admin` return codes, plus two outcomes the mod decides without calling it:
 * [NOT_EMBEDDED] and [NATIVE_OUTDATED].
 */
enum class AdminResult {
    APPLIED,
    NOTHING_TO_REVOKE,
    PLAYER_NOT_FOUND,
    FAILED,
    NOT_EMBEDDED,
    NATIVE_OUTDATED;

    companion object {
        fun fromCode(code: Int): AdminResult = when (code) {
            0 -> APPLIED
            1 -> NOTHING_TO_REVOKE
            2 -> PLAYER_NOT_FOUND
            else -> FAILED
        }
    }
}
