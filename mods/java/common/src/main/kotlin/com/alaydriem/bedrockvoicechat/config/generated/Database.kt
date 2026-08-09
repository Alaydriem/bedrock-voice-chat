package com.alaydriem.bedrockvoicechat.config.generated

import com.google.gson.annotations.SerializedName

// Generated from the Rust `ApplicationConfig`. Do not edit.
// Regenerate with:
//   UPDATE_KOTLIN_CONFIG=1 cargo nextest run kotlin_export
class Database {
    @SerializedName("scheme")
    var scheme: String? = null

    @SerializedName("database")
    var database: String? = null

    @SerializedName("host")
    var host: String? = null

    @SerializedName("username")
    var username: String? = null

    @SerializedName("password")
    var password: String? = null

    @SerializedName("port")
    var port: Long? = null

    @SerializedName("ssl_mode")
    var sslMode: String? = null

    @SerializedName("ssl_root_cert")
    var sslRootCert: String? = null

    @SerializedName("ssl_cert")
    var sslCert: String? = null

    @SerializedName("ssl_key")
    var sslKey: String? = null

}
