package com.alaydriem.bedrockvoicechat.svc

import com.alaydriem.bedrockvoicechat.svc.link.LinkEntry
import com.alaydriem.bedrockvoicechat.svc.link.SvcGroupType
import com.alaydriem.bedrockvoicechat.svc.link.SvcGroupView
import com.alaydriem.bedrockvoicechat.svc.link.SvcLinkTable
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Test
import java.util.UUID

class SvcGroupGateTest {
    private val table = SvcLinkTable()
    private val gate = SvcGroupGate(table)

    private fun group(type: SvcGroupType, password: Boolean = false) =
        SvcGroupView(UUID.randomUUID(), "Raid", type, password, hidden = false)

    private fun linked(type: SvcGroupType): SvcGroupView {
        val view = group(type)
        table.put(LinkEntry(view.id, "ABCD", view.membersOnly, LinkEntry.Origin.SVC))
        return view
    }

    @Test
    fun `no_group_is_proximity`() {
        assertEquals(GroupRoute.Proximity, gate.route(null))
    }

    // OPEN speech is heard nearby on both sides; the marker lets group members far apart hear it.
    @Test
    fun `linked_open_is_group_not_members_only`() {
        assertEquals(GroupRoute.Group("ABCD", membersOnly = false), gate.route(linked(SvcGroupType.OPEN)))
    }

    @Test
    fun `linked_normal_is_group_members_only`() {
        assertEquals(GroupRoute.Group("ABCD", membersOnly = true), gate.route(linked(SvcGroupType.NORMAL)))
    }

    @Test
    fun `linked isolated is group members only`() {
        assertEquals(GroupRoute.Group("ABCD", membersOnly = true), gate.route(linked(SvcGroupType.ISOLATED)))
    }

    @Test
    fun `unlinked_open_is_proximity`() {
        assertEquals(GroupRoute.Proximity, gate.route(group(SvcGroupType.OPEN)))
    }

    // A password or hidden OPEN group is not linked, and OPEN speech is heard nearby anyway.
    @Test
    fun `unlinked open with a password is proximity`() {
        assertEquals(GroupRoute.Proximity, gate.route(group(SvcGroupType.OPEN, password = true)))
    }

    // Sent as proximity audio, a NORMAL group's speech would reach every nearby Bedrock player.
    @Test
    fun `unlinked_normal_is_dropped`() {
        assertEquals(GroupRoute.Drop, gate.route(group(SvcGroupType.NORMAL)))
    }

    @Test
    fun `unlinked_isolated_is_dropped`() {
        assertEquals(GroupRoute.Drop, gate.route(group(SvcGroupType.ISOLATED)))
    }
}
