package com.alaydriem.bedrockvoicechat.admin

/**
 * The outcome of a `bvc_admin` call, [NOT_EMBEDDED] when there was no server to call, or
 * [NATIVE_OUTDATED] when the loaded native library has no `bvc_admin` export.
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
