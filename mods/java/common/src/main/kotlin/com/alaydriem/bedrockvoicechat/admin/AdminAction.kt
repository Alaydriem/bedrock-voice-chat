package com.alaydriem.bedrockvoicechat.admin

import com.google.gson.annotations.SerializedName

/** Mirrors common's `AdminAction`; [value] is also the `/bvc admin` subcommand. */
enum class AdminAction(val value: String) {
    @SerializedName("grant")
    GRANT("grant"),

    @SerializedName("revoke")
    REVOKE("revoke"),

    @SerializedName("deny")
    DENY("deny"),
}
