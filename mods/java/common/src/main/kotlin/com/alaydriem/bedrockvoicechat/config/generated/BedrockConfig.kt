package com.alaydriem.bedrockvoicechat.config.generated

import com.google.gson.annotations.SerializedName

// Generated from the Rust `ApplicationConfig`. Do not edit.
// Regenerate with:
//   UPDATE_KOTLIN_CONFIG=1 cargo nextest run kotlin_export
class BedrockConfig {
    @SerializedName("enabled")
    var enabled: Boolean? = null

    @SerializedName("transfer_port")
    var transferPort: Int? = null

    @SerializedName("transfer_target_port")
    var transferTargetPort: Int? = null

    @SerializedName("transfer_cache_ttl_secs")
    var transferCacheTtlSecs: Long? = null

    @SerializedName("proxy_event_freshness_threshold_secs")
    var proxyEventFreshnessThresholdSecs: Long? = null

}
