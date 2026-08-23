package com.alaydriem.bedrockvoicechat.config.generated

import com.google.gson.annotations.SerializedName

// Generated from the Rust `ApplicationConfig`. Do not edit.
// Regenerate with:
//   UPDATE_KOTLIN_CONFIG=1 cargo nextest run kotlin_export
class Acme {
    @SerializedName("email")
    var email: String? = null

    @SerializedName("provider")
    var provider: String? = null

    @SerializedName("api_token")
    var apiToken: String? = null

    @SerializedName("server_url")
    var serverUrl: String? = null

    @SerializedName("username")
    var username: String? = null

    @SerializedName("password")
    var password: String? = null

    @SerializedName("subdomain")
    var subdomain: String? = null

    @SerializedName("directory")
    var directory: String? = null

    @SerializedName("domains")
    var domains: List<String>? = null

}
