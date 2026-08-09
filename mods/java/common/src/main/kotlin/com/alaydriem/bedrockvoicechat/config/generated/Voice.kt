package com.alaydriem.bedrockvoicechat.config.generated

import com.google.gson.annotations.SerializedName

// Generated from the Rust `ApplicationConfig`. Do not edit.
// Regenerate with:
//   UPDATE_KOTLIN_CONFIG=1 cargo nextest run kotlin_export
class Voice {
    @SerializedName("datagram_send_capacity")
    var datagramSendCapacity: Long? = null

    @SerializedName("datagram_recv_capacity")
    var datagramRecvCapacity: Long? = null

    @SerializedName("spatial_audio")
    var spatialAudio: SpatialAudioConfig? = null

}
