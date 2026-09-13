package com.alaydriem.bedrockvoicechat.config.generated

import com.google.gson.annotations.SerializedName

// Generated from the Rust `ApplicationConfig`. Do not edit.
// Regenerate with:
//   UPDATE_KOTLIN_CONFIG=1 cargo nextest run kotlin_export
class BedrockConfig {
    @SerializedName("proxy_event_freshness_threshold_secs")
    var proxyEventFreshnessThresholdSecs: Long? = null

}
