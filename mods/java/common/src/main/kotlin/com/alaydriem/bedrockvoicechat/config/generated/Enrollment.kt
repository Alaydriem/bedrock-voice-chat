package com.alaydriem.bedrockvoicechat.config.generated

import com.google.gson.annotations.SerializedName

// Generated from the Rust `ApplicationConfig`. Do not edit.
// Regenerate with:
//   UPDATE_KOTLIN_CONFIG=1 cargo nextest run kotlin_export
class Enrollment {
    @SerializedName("token")
    var token: String? = null

    @SerializedName("address")
    var address: String? = null

}
