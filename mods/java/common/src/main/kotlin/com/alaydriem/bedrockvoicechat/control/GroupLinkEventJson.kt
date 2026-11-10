package com.alaydriem.bedrockvoicechat.control

import com.google.gson.annotations.SerializedName

/**
 * One BVC channel change, mirroring common's `GroupLinkEvent`. [player] is a gamertag. [kind] is
 * null for a kind a newer server sends and this build does not know.
 */
data class GroupLinkEventJson(
    val seq: Long,
    val kind: GroupLinkEventKind?,
    val channel: String,
    @SerializedName("channel_name") val channelName: String?,
    val player: String?,
    @SerializedName("members_only") val membersOnly: Boolean,
)
