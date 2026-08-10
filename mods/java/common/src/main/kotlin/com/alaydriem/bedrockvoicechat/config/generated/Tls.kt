package com.alaydriem.bedrockvoicechat.config.generated

import com.google.gson.annotations.SerializedName

// Generated from the Rust `ApplicationConfig`. Do not edit.
// Regenerate with:
//   UPDATE_KOTLIN_CONFIG=1 cargo nextest run kotlin_export
class Tls {
    @SerializedName("certificate")
    var certificate: String? = null

    @SerializedName("key")
    var key: String? = null

    @SerializedName("certs_path")
    var certsPath: String? = null

    @SerializedName("names")
    var names: List<String>? = null

    @SerializedName("ips")
    var ips: List<String>? = null

    @SerializedName("acme")
    var acme: Acme? = null

}
