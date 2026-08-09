package com.alaydriem.bedrockvoicechat.config.generated

import com.google.gson.annotations.SerializedName

// Generated from the Rust `ApplicationConfig`. Do not edit.
// Regenerate with:
//   UPDATE_KOTLIN_CONFIG=1 cargo nextest run kotlin_export
class BedrockDnsConfig {
    @SerializedName("enabled")
    var enabled: Boolean? = null

    @SerializedName("port")
    var port: Int? = null

    @SerializedName("upstream")
    var upstream: List<String>? = null

    @SerializedName("override_host")
    var overrideHost: String? = null

    @SerializedName("rate_limit_per_sec")
    var rateLimitPerSec: Long? = null

}
