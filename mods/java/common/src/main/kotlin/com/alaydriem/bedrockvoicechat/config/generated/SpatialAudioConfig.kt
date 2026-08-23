package com.alaydriem.bedrockvoicechat.config.generated

import com.google.gson.annotations.SerializedName

// Generated from the Rust `ApplicationConfig`. Do not edit.
// Regenerate with:
//   UPDATE_KOTLIN_CONFIG=1 cargo nextest run kotlin_export
class SpatialAudioConfig {
    @SerializedName("broadcast_range")
    var broadcastRange: Float? = null

    @SerializedName("close_threshold")
    var closeThreshold: Float? = null

    @SerializedName("falloff_distance")
    var falloffDistance: Float? = null

    @SerializedName("steepen_start")
    var steepenStart: Float? = null

    @SerializedName("deafen_distance")
    var deafenDistance: Float? = null

    @SerializedName("panning_start")
    var panningStart: Float? = null

    @SerializedName("max_attenuation_db")
    var maxAttenuationDb: Float? = null

}
