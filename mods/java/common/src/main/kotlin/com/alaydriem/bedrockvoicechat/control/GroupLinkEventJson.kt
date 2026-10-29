package com.alaydriem.bedrockvoicechat.control

import com.google.gson.annotations.SerializedName

/** One BVC channel change, mirroring common's `GroupLinkEvent`. [player] is a gamertag. */
data class GroupLinkEventJson(
    val seq: Long,
    val kind: GroupLinkEventKind,
    val channel: String,
    @SerializedName("channel_name") val channelName: String?,
    val player: String?,
    @SerializedName("members_only") val membersOnly: Boolean,
)
