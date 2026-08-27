package com.alaydriem.bedrockvoicechat.config.generated

import com.google.gson.annotations.SerializedName

// Generated from the Rust `ApplicationConfig`. Do not edit.
// Regenerate with:
//   UPDATE_KOTLIN_CONFIG=1 cargo nextest run kotlin_export
class Server {
    @SerializedName("listen")
    var listen: String? = null

    @SerializedName("port")
    var port: Long? = null

    @SerializedName("quic_port")
    var quicPort: Long? = null

    @SerializedName("advertised_quic_ports")
    var advertisedQuicPorts: List<Long>? = null

    @SerializedName("assets_path")
    var assetsPath: String? = null

    @SerializedName("tls")
    var tls: Tls? = null

    @SerializedName("minecraft")
    var minecraft: Minecraft? = null

    @SerializedName("features")
    var features: Features? = null

    @SerializedName("bedrock")
    var bedrock: BedrockConfig? = null

    @SerializedName("age")
    var age: Age? = null

    @SerializedName("peers")
    var peers: Map<String, PeerConfig>? = null

    @SerializedName("enrollment")
    var enrollment: Enrollment? = null

    @SerializedName("peer_port")
    var peerPort: Int? = null

}
