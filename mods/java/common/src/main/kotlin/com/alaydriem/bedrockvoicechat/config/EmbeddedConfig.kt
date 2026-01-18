package com.alaydriem.bedrockvoicechat.config

import com.google.gson.annotations.SerializedName

/**
 * Configuration for embedded BVC server mode.
 * Only used when useEmbeddedServer is true.
 *
 * Two certificate systems are required:
 * 1. HTTPS TLS (tls-certificate, tls-key) - Must be signed by trusted CA (e.g., Let's Encrypt)
 * 2. QUIC mTLS - Auto-generated CA for client authentication (stored in data directory)
 */
class EmbeddedConfig {
    @SerializedName(value = "http-port", alternate = ["httpPort"])
    var httpPort: Int = 8444

    @SerializedName(value = "quic-port", alternate = ["quicPort"])
    var quicPort: Int = 8443

    @SerializedName(value = "public-addr", alternate = ["publicAddr"])
    var publicAddr: String = "0.0.0.0"

    @SerializedName(value = "broadcast-range", alternate = ["broadcastRange"])
    var broadcastRange: Float = 32.0f

    /** Path to TLS certificate file (must be signed by trusted CA) */
    @SerializedName(value = "tls-certificate", alternate = ["tlsCertificate"])
    var tlsCertificate: String = ""

    /** Path to TLS private key file */
    @SerializedName(value = "tls-key", alternate = ["tlsKey"])
    var tlsKey: String = ""

    @SerializedName(value = "tls-names", alternate = ["tlsNames"])
    var tlsNames: List<String> = listOf("localhost", "127.0.0.1")

    @SerializedName(value = "tls-ips", alternate = ["tlsIps"])
    var tlsIps: List<String> = listOf("127.0.0.1")

    @SerializedName(value = "log-level", alternate = ["logLevel"])
    var logLevel: String = "info"

    /** Check if TLS certificate paths are configured */
    fun hasTlsCertificates(): Boolean = tlsCertificate.isNotBlank() && tlsKey.isNotBlank()
}
