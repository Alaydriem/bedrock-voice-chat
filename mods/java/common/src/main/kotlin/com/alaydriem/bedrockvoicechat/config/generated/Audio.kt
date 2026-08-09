package com.alaydriem.bedrockvoicechat.config.generated

import com.google.gson.annotations.SerializedName

// Generated from the Rust `ApplicationConfig`. Do not edit.
// Regenerate with:
//   UPDATE_KOTLIN_CONFIG=1 cargo nextest run kotlin_export
class Audio {
    @SerializedName("file_path")
    var filePath: String? = null

    @SerializedName("max_concurrent_per_uuid")
    var maxConcurrentPerUuid: Long? = null

}
