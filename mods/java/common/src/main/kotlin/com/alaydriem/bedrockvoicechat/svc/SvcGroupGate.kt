package com.alaydriem.bedrockvoicechat.svc

import com.alaydriem.bedrockvoicechat.svc.link.SvcGroupView
import com.alaydriem.bedrockvoicechat.svc.link.SvcLinkTable

/**
 * Decides where a Java speaker's speech may go, by their SVC group.
 *
 * SVC keeps NORMAL and ISOLATED speech inside the group. Sent as proximity audio it would reach
 * every nearby Bedrock player, so it crosses only with its group marker, and only when the group
 * is linked; otherwise it is dropped.
 */
class SvcGroupGate(private val table: SvcLinkTable) {
    fun route(group: SvcGroupView?): GroupRoute {
        if (group == null) return GroupRoute.Proximity
        table.bySvc(group.id)?.let { return GroupRoute.Group(it.bvcChannel, it.membersOnly) }
        return if (group.membersOnly) GroupRoute.Drop else GroupRoute.Proximity
    }
}
