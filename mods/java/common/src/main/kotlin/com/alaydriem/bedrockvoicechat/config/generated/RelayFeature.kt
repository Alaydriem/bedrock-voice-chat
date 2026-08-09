package com.alaydriem.bedrockvoicechat.config.generated

import com.google.gson.annotations.SerializedName

// Generated from the Rust `ApplicationConfig`. Do not edit.
// Regenerate with:
//   UPDATE_KOTLIN_CONFIG=1 cargo nextest run kotlin_export
class RelayFeature {
    @SerializedName("announce_interval_secs")
    var announceIntervalSecs: Long? = null

    @SerializedName("orchestration_interval_secs")
    var orchestrationIntervalSecs: Long? = null

    @SerializedName("idle_timeout_secs")
    var idleTimeoutSecs: Long? = null

}
