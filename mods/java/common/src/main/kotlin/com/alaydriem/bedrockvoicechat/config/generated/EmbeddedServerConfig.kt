package com.alaydriem.bedrockvoicechat.config.generated

import com.google.gson.annotations.SerializedName

// Generated from the Rust `ApplicationConfig`. Do not edit.
// Regenerate with:
//   UPDATE_KOTLIN_CONFIG=1 cargo nextest run kotlin_export
class EmbeddedServerConfig {
    @SerializedName("database")
    var database: Database? = null

    @SerializedName("server")
    var server: Server? = null

    @SerializedName("log")
    var log: Logger? = null

    @SerializedName("voice")
    var voice: Voice? = null

    @SerializedName("audio")
    var audio: Audio? = null

    @SerializedName("permissions")
    var permissions: Permissions? = null

}
