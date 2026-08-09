package com.alaydriem.bedrockvoicechat.config.generated

import com.google.gson.annotations.SerializedName

// Generated from the Rust `ApplicationConfig`. Do not edit.
// Regenerate with:
//   UPDATE_KOTLIN_CONFIG=1 cargo nextest run kotlin_export
class Features {
    @SerializedName("relay")
    var relay: RelayFeature? = null

    @SerializedName("telemetry")
    var telemetry: Boolean? = null

}
