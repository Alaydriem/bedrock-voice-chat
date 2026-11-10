package com.alaydriem.bedrockvoicechat.svc.link

import java.util.UUID

/** An SVC group as the linker sees it. Read on the server thread and passed on as a value. */
data class SvcGroupView(
    val id: UUID,
    val name: String,
    val type: SvcGroupType,
    val hasPassword: Boolean,
    val hidden: Boolean,
) {
    /** Password and hidden groups are private to SVC and never linked. */
    val linkable: Boolean
        get() = !hasPassword && !hidden

    val membersOnly: Boolean
        get() = type != SvcGroupType.OPEN
}
