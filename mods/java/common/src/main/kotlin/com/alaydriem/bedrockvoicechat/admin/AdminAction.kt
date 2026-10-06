package com.alaydriem.bedrockvoicechat.admin

/** What `/bvc admin` does to a player's `admin` permission; [wire] is the FFI action name. */
enum class AdminAction(val wire: String) {
    GRANT("grant"),
    REVOKE("revoke"),
    DENY("deny"),
}
