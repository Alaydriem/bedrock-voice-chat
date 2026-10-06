package com.alaydriem.bedrockvoicechat.admin

/** Something `/bvc admin` can change a player's `admin` permission on: the embedded server. */
interface AdminTarget {
    fun admin(gamertag: String, action: AdminAction): AdminResult

    /** The last native error, for the thread that made the failing call. */
    fun lastError(): String?
}
