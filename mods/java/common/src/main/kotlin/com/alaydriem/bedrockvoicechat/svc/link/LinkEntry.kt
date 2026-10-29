package com.alaydriem.bedrockvoicechat.svc.link

import java.util.UUID

/**
 * One SVC group linked to one BVC channel. [origin] is the side that owns the group's existence:
 * a group ends when its origin side ends it. [membersOnly] is true for NORMAL and ISOLATED groups.
 *
 * [pendingRemove] marks a BVC-origin group whose SVC side refused removal once; resync retries it
 * a single time.
 */
data class LinkEntry(
    val svcGroup: UUID,
    val bvcChannel: String,
    val membersOnly: Boolean,
    val origin: Origin,
    val pendingRemove: Boolean = false,
) {
    enum class Origin { SVC, BVC }
}
