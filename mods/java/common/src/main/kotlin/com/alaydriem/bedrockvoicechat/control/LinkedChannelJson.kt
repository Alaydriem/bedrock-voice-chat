package com.alaydriem.bedrockvoicechat.control

import com.google.gson.annotations.SerializedName

/**
 * A non-protected BVC channel, mirroring common's `LinkedChannelView`. [players] are gamertags.
 *
 * [clientless] names the members that hold no live BVC client. It is null from a server that
 * predates the field: Gson does not apply a Kotlin default to a field the JSON leaves out.
 */
data class LinkedChannelJson(
    val id: String,
    val name: String,
    val players: List<String>,
    @SerializedName("members_only") val membersOnly: Boolean,
    val clientless: List<String>? = null,
)
