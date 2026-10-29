package com.alaydriem.bedrockvoicechat.svc.link

import java.util.UUID

/**
 * SVC's group events as plain values, read on the server thread and handed to the linker.
 * [creator] is null for a group another plugin made through the API.
 */
interface SvcLinkEvents {
    fun created(group: SvcGroupView, creator: UUID?)
    fun joined(group: UUID, player: UUID)
    fun left(player: UUID, group: UUID?)
    fun removed(group: UUID)
}
