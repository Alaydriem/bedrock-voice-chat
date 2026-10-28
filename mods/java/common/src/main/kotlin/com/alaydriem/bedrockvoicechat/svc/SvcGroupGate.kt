package com.alaydriem.bedrockvoicechat.svc

import de.maxhenkel.voicechat.api.Group

/**
 * Whose speech may leave Simple Voice Chat.
 *
 * Everything the bridge sends reaches BVC as proximity audio. That matches SVC only
 * for a speaker outside any group or in an OPEN group, who nearby players hear too.
 * NORMAL and ISOLATED groups are heard by their members alone, so proximity
 * delivery would play them to nearby Bedrock players SVC itself keeps out.
 */
object SvcGroupGate {

    fun bridges(group: Group?): Boolean = group == null || group.type == Group.Type.OPEN
}
