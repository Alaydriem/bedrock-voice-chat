package com.alaydriem.bedrockvoicechat.svc

import com.alaydriem.bedrockvoicechat.svc.link.SvcGroupType
import com.alaydriem.bedrockvoicechat.svc.link.SvcGroupView
import de.maxhenkel.voicechat.api.Group

/**
 * Reads an SVC group into the bridge's own value. Names SVC types, so it loads only once
 * [SvcAvailability] says they are present.
 */
object SvcGroupViews {
    // A type this build does not know is treated as private rather than OPEN: guessing OPEN would
    // send a private group's speech to nearby players.
    fun of(group: Group): SvcGroupView = SvcGroupView(
        id = group.id,
        name = group.name,
        type = when (group.type) {
            Group.Type.OPEN -> SvcGroupType.OPEN
            Group.Type.ISOLATED -> SvcGroupType.ISOLATED
            else -> SvcGroupType.NORMAL
        },
        hasPassword = group.hasPassword(),
        hidden = group.isHidden,
    )
}
