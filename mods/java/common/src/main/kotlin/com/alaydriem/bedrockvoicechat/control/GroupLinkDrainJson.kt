package com.alaydriem.bedrockvoicechat.control

/**
 * The BVC channel changes after a sequence number, mirroring common's `GroupLinkDrain`.
 * [reset] means events were missed or the server restarted, so the bridge resyncs.
 *
 * [latest] is the highest sequence number the feed has given out. It is 0 for a feed with no
 * events, and also from a server that predates the field.
 */
data class GroupLinkDrainJson(
    val events: List<GroupLinkEventJson>,
    val reset: Boolean,
    val epoch: Long = 0,
    val latest: Long = 0,
)
