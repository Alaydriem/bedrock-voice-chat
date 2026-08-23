package com.alaydriem.bedrockvoicechat.svc

/**
 * Who a speaker is and where they are, at the moment a frame is built.
 *
 * Name and position together because both are read from the same platform lookup,
 * and reading them separately would let a player move between the two.
 *
 * `dimension` is the api form: "overworld", "nether", "the_end".
 */
data class SpeakerSnapshot(
    val name: String,
    val x: Float,
    val y: Float,
    val z: Float,
    val dimension: String
)
