package com.alaydriem.bedrockvoicechat.control

import com.google.gson.annotations.SerializedName

/** A non-protected BVC channel, mirroring common's `LinkedChannelView`. [players] are gamertags. */
data class LinkedChannelJson(
    val id: String,
    val name: String,
    val players: List<String>,
    @SerializedName("members_only") val membersOnly: Boolean,
)
