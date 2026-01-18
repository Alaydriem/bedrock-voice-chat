package com.alaydriem.bedrockvoicechat.config

import com.google.gson.annotations.SerializedName

/**
 * Configuration for embedded BVC server mode.
 * Only used when useEmbeddedServer is true.
 */
class EmbeddedConfig {
    @SerializedName(value = "http-port", alternate = ["httpPort"])
    var httpPort: Int = 443

    @SerializedName(value = "quic-port", alternate = ["quicPort"])
    var quicPort: Int = 8443

    @SerializedName(value = "public-addr", alternate = ["publicAddr"])
    var publicAddr: String = "127.0.0.1"

    @SerializedName(value = "broadcast-range", alternate = ["broadcastRange"])
    var broadcastRange: Float = 32.0f

    @SerializedName(value = "tls-names", alternate = ["tlsNames"])
    var tlsNames: List<String> = listOf("localhost", "127.0.0.1")

    @SerializedName(value = "tls-ips", alternate = ["tlsIps"])
    var tlsIps: List<String> = listOf("127.0.0.1")

    @SerializedName(value = "log-level", alternate = ["logLevel"])
    var logLevel: String = "info"
}
