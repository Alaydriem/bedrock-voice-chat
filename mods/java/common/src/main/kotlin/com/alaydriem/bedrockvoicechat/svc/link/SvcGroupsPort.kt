package com.alaydriem.bedrockvoicechat.svc.link

import java.util.UUID

/**
 * SVC's group API as the linker needs it. The implementation runs each call on the server
 * thread and waits for it; the linker calls it from the link thread.
 */
interface SvcGroupsPort {
    /** SVC's `enable_groups` setting. */
    fun groupsEnabled(): Boolean
    fun groups(): List<SvcGroupView>
    /** SVC has no member list; this sweeps every connection. */
    fun membersOf(group: UUID): List<UUID>
    fun create(name: String, type: SvcGroupType, persistent: Boolean): UUID?
    /** False when another plugin cancelled the join. */
    fun setGroup(player: UUID, group: UUID?): Boolean
    /** False while any player state still names the group. */
    fun remove(group: UUID): Boolean
}
