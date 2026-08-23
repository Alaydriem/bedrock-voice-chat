package com.alaydriem.bedrockvoicechat.config

import com.google.gson.JsonObject

/**
 * Detects the flat embedded-config keys the mod used before the config mirrored
 * the server's own shape.
 *
 * Gson discards keys it does not recognise, so without this an upgraded config
 * would start a server on defaults with none of the operator's settings applied
 * and nothing written to the log.
 *
 * The table is frozen. It describes a shape that no longer exists, so it never
 * grows.
 */
object LegacyEmbeddedKeys {
    private val REPLACEMENTS = mapOf(
        "http-port" to "server.port",
        "httpPort" to "server.port",
        "quic-port" to "server.quic_port",
        "quicPort" to "server.quic_port",
        "broadcast-range" to "voice.spatial_audio.broadcast_range",
        "broadcastRange" to "voice.spatial_audio.broadcast_range",
        "tls-certificate" to "server.tls.certificate",
        "tlsCertificate" to "server.tls.certificate",
        "tls-key" to "server.tls.key",
        "tlsKey" to "server.tls.key",
        "tls-names" to "server.tls.names",
        "tlsNames" to "server.tls.names",
        "tls-ips" to "server.tls.ips",
        "tlsIps" to "server.tls.ips",
        "log-level" to "log.level",
        "logLevel" to "log.level",
        "assets-path" to "server.assets_path",
        "assetsPath" to "server.assets_path",
        "allow-audio-upload" to "permissions.defaults.audio_upload",
        "allowAudioUpload" to "permissions.defaults.audio_upload",
        "allow-audio-delete" to "permissions.defaults.audio_delete",
        "allowAudioDelete" to "permissions.defaults.audio_delete"
    )

    fun detect(embedded: JsonObject?): List<String> {
        if (embedded == null) {
            return emptyList()
        }

        return embedded.keySet().mapNotNull { key -> REPLACEMENTS[key]?.let { "$key -> $it" } }
    }
}
