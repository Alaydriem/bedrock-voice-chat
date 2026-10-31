package com.alaydriem.bedrockvoicechat.control

/**
 * The BVC channel changes after a sequence number, mirroring common's `GroupLinkDrain`.
 * [reset] means events were missed or the server restarted, so the bridge resyncs.
 */
data class GroupLinkDrainJson(
    val events: List<GroupLinkEventJson>,
    val reset: Boolean,
    val epoch: Long = 0,
)
