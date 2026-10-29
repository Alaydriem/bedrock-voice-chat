package com.alaydriem.bedrockvoicechat.svc.link

import com.alaydriem.bedrockvoicechat.control.GroupLinkDrainJson
import com.alaydriem.bedrockvoicechat.control.GroupLinkEventJson
import com.alaydriem.bedrockvoicechat.control.GroupLinkEventKind
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertNotNull
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import java.util.UUID

class SvcGroupLinkerTest {
    // Steve plays Java with the SVC mod, Bob plays Bedrock with the BVC client, and Nobody has
    // neither voice client.
    private val steve = UUID.randomUUID()
    private val bob = UUID.randomUUID()
    private val nobody = UUID.randomUUID()
    private val names = mapOf(steve to "Steve", bob to "Bob", nobody to "Nobody")

    private val svc = FakeSvcGroups()
    private val bvc = FakeGroupLinkApi()
    private val table = SvcLinkTable()
    private val refusals = LinkRefusals()
    private val linker = SvcGroupLinker(
        svc = svc,
        bvc = bvc,
        table = table,
        sides = PlayerSides(hasLiveBvcClient = { it == bob }, hasSvcInstalled = { it == steve }),
        echo = EchoGuard(),
        refusals = refusals,
        actorIdOf = { names[it] },
        playerOf = { name -> names.entries.firstOrNull { it.value == name }?.key },
    )

    private var seq = 0L

    private fun bvcEvents(vararg events: GroupLinkEventJson, reset: Boolean = false, epoch: Long = 1L) {
        bvc.nextDrain = GroupLinkDrainJson(events.toList(), reset, epoch)
        linker.pollBvc()
    }

    private fun event(kind: GroupLinkEventKind, channel: String, player: String? = null, name: String? = "Raid") =
        GroupLinkEventJson(++seq, kind, channel, name, player, false)

    // A BVC channel made through the bridge's own SVC link, as if `onSvcCreate` had run.
    private fun linkedSvcGroup(type: SvcGroupType = SvcGroupType.OPEN): Pair<SvcGroupView, String> {
        val view = svc.add("Raid", type)
        linker.onSvcCreate(view, steve)
        return view to table.bySvc(view.id)!!.bvcChannel
    }

    @Test
    fun `svc_open_group_links_open`() {
        val (view, channel) = linkedSvcGroup(SvcGroupType.OPEN)

        assertEquals(listOf("createLinked:Steve:Raid:false"), bvc.calls)
        assertEquals(LinkEntry(view.id, channel, false, LinkEntry.Origin.SVC), table.bySvc(view.id))
    }

    @Test
    fun `svc_normal_group_links_members_only`() {
        val (view, _) = linkedSvcGroup(SvcGroupType.NORMAL)

        assertEquals(listOf("createLinked:Steve:Raid:true"), bvc.calls)
        assertTrue(table.bySvc(view.id)!!.membersOnly)
    }

    @Test
    fun `svc_password_group_is_not_linked`() {
        linker.onSvcCreate(svc.add("Secret", password = true), steve)

        assertTrue(bvc.calls.isEmpty())
        assertTrue(table.all().isEmpty())
    }

    @Test
    fun `svc_hidden_group_is_not_linked`() {
        linker.onSvcCreate(svc.add("Hidden", hidden = true), steve)

        assertTrue(bvc.calls.isEmpty())
    }

    // SVC's empty-group cleanup must not end a group BVC owns.
    @Test
    fun `bvc_group_creates_persistent_open_svc_group`() {
        val channel = bvc.add("Raid", "Bob")

        bvcEvents(event(GroupLinkEventKind.Create, channel, "Bob"))

        val entry = table.byBvc(channel)!!
        assertEquals(LinkEntry.Origin.BVC, entry.origin)
        assertEquals(SvcGroupType.OPEN, svc.groups[entry.svcGroup]!!.type)
        assertTrue(entry.svcGroup in svc.persistent)
    }

    // The SVC group the bridge creates for a BVC group fires CreateGroupEvent back into it.
    @Test
    fun `own_create_event_is_ignored`() {
        val channel = bvc.add("Raid", "Bob")
        bvcEvents(event(GroupLinkEventKind.Create, channel, "Bob"))
        val created = svc.groups[table.byBvc(channel)!!.svcGroup]!!

        linker.onSvcCreate(created, bob)

        assertTrue(bvc.calls.none { it.startsWith("createLinked") })
    }

    // Likewise the BVC channel the bridge creates for an SVC group comes back as a Create event.
    @Test
    fun `own bvc create event is ignored`() {
        val (_, channel) = linkedSvcGroup()

        bvcEvents(event(GroupLinkEventKind.Create, channel, "Steve"))

        assertEquals(1, svc.groups.size)
    }

    @Test
    fun `svc_join_by_svc_player_joins_bvc`() {
        val (view, channel) = linkedSvcGroup()

        linker.onSvcJoin(view.id, steve)

        assertEquals("join:Steve:$channel", bvc.calls.last())
    }

    // Bob's membership is owned by BVC; an SVC change for him is a correction, not an instruction.
    @Test
    fun `svc_join_by_bvc_side_player_is_ignored`() {
        val (view, _) = linkedSvcGroup()

        linker.onSvcJoin(view.id, bob)

        assertTrue(bvc.calls.none { it.startsWith("join") })
    }

    @Test
    fun `bvc_join_by_bvc_player_sets_svc_group`() {
        val (view, channel) = linkedSvcGroup()

        bvcEvents(event(GroupLinkEventKind.Join, channel, "Bob"))

        assertEquals(view.id, svc.memberOf[bob])
    }

    @Test
    fun `none_side_player_is_ignored`() {
        val (view, channel) = linkedSvcGroup()

        linker.onSvcJoin(view.id, nobody)
        bvcEvents(event(GroupLinkEventKind.Join, channel, "Nobody"))

        assertTrue(bvc.calls.none { it.startsWith("join") })
        assertNull(svc.memberOf[nobody])
    }

    @Test
    fun `bvc leave clears the svc group`() {
        val (_, channel) = linkedSvcGroup()
        bvcEvents(event(GroupLinkEventKind.Join, channel, "Bob"))

        bvcEvents(event(GroupLinkEventKind.Leave, channel, "Bob"))

        assertNull(svc.memberOf[bob])
    }

    @Test
    fun `svc leave by svc player leaves bvc`() {
        val (view, _) = linkedSvcGroup()

        linker.onSvcLeave(steve, view.id)

        assertEquals("leave:Steve", bvc.calls.last())
    }

    // Another plugin cancelled the join. Resync must not hammer it; a new membership change may.
    @Test
    fun `cancelled_join_is_not_retried_until_membership_changes`() {
        val (view, channel) = linkedSvcGroup()
        svc.cancelJoinsFor.add(bob)
        bvcEvents(event(GroupLinkEventKind.Join, channel, "Bob"))
        bvc.channels[channel] = bvc.channels[channel]!!.copy(players = listOf("Bob"))
        assertTrue(refusals.isRefused(bob, view.id))

        val attemptsBefore = svc.calls.count { it == "setGroup:$bob:${view.id}" }
        linker.resync()
        assertEquals(attemptsBefore, svc.calls.count { it == "setGroup:$bob:${view.id}" })

        svc.cancelJoinsFor.clear()
        bvcEvents(event(GroupLinkEventKind.Join, channel, "Bob"))
        assertEquals(view.id, svc.memberOf[bob])
    }

    @Test
    fun `svc_remove_closes_bvc_channel`() {
        val (view, channel) = linkedSvcGroup()

        linker.onSvcRemove(view.id)

        assertEquals("close:$channel", bvc.calls.last())
        assertNull(table.bySvc(view.id))
    }

    // SVC refuses removeGroup while any player state names the group.
    @Test
    fun `teardown_moves_members_out_before_remove`() {
        val channel = bvc.add("Raid", "Bob")
        bvcEvents(event(GroupLinkEventKind.Create, channel, "Bob"))
        val group = table.byBvc(channel)!!.svcGroup
        svc.memberOf[steve] = group
        svc.memberOf[bob] = group

        bvcEvents(event(GroupLinkEventKind.Delete, channel, name = null))

        assertFalse(svc.groups.containsKey(group))
        assertNull(table.byBvc(channel))
        val removeAt = svc.calls.indexOf("remove:$group")
        assertTrue(svc.calls.indexOf("setGroup:$steve:null") in 0 until removeAt)
        assertTrue(svc.calls.indexOf("setGroup:$bob:null") in 0 until removeAt)
    }

    // An orphan SVC group is an anti-goal, so one failure is retried; a second is given up on.
    @Test
    fun `remove_failure_retries_once`() {
        val channel = bvc.add("Raid", "Bob")
        bvcEvents(event(GroupLinkEventKind.Create, channel, "Bob"))
        val group = table.byBvc(channel)!!.svcGroup
        svc.removeFailures = 1

        bvc.channels.remove(channel)
        bvcEvents(event(GroupLinkEventKind.Delete, channel, name = null))
        assertTrue(table.byBvc(channel)!!.pendingRemove)

        linker.resync()
        assertFalse(svc.groups.containsKey(group))
        assertNull(table.byBvc(channel))
        assertEquals(2, svc.removeCalls.count { it == group })

        linker.resync()
        assertEquals(2, svc.removeCalls.count { it == group })
    }

    @Test
    fun `a second remove failure gives up`() {
        val channel = bvc.add("Raid", "Bob")
        bvcEvents(event(GroupLinkEventKind.Create, channel, "Bob"))
        svc.removeFailures = 2

        bvc.channels.remove(channel)
        bvcEvents(event(GroupLinkEventKind.Delete, channel, name = null))
        linker.resync()

        assertNull(table.byBvc(channel))
    }

    @Test
    fun `protect_unlinks_and_tears_down_svc`() {
        val (view, channel) = linkedSvcGroup()
        svc.memberOf[bob] = view.id

        bvcEvents(event(GroupLinkEventKind.Protected, channel, name = null))

        assertNull(table.byBvc(channel))
        assertFalse(svc.groups.containsKey(view.id))
        assertNull(svc.memberOf[bob])
    }

    // A Java server restart loses SVC's groups; BVC still owns its own and re-creates them.
    @Test
    fun `resync_recreates_missing_bvc_origin_group`() {
        val channel = bvc.add("Raid", "Bob")
        bvcEvents(event(GroupLinkEventKind.Create, channel, "Bob"))
        val lost = table.byBvc(channel)!!.svcGroup
        svc.groups.remove(lost)
        svc.memberOf.remove(bob)

        linker.resync()

        val recreated = table.byBvc(channel)!!.svcGroup
        assertTrue(recreated != lost)
        assertNotNull(svc.groups[recreated])
        assertEquals(recreated, svc.memberOf[bob])
    }

    @Test
    fun `resync_closes_bvc_side_of_vanished_svc_group`() {
        val (view, channel) = linkedSvcGroup()
        svc.groups.remove(view.id)

        linker.resync()

        assertTrue("close:$channel" in bvc.calls)
        assertNull(table.byBvc(channel))
    }

    @Test
    fun `resync_matches_by_name_on_empty_table`() {
        val view = svc.add("Raid", SvcGroupType.NORMAL)
        svc.memberOf[steve] = view.id
        val channel = bvc.add("Raid", "Steve", membersOnly = true)

        linker.resync()

        assertEquals(channel, table.bySvc(view.id)!!.bvcChannel)
        assertTrue(bvc.calls.none { it.startsWith("createLinked") })
        assertTrue(svc.calls.none { it.startsWith("create:") })
    }

    @Test
    fun `resync links leftover groups on both sides as new`() {
        val view = svc.add("Mine")
        svc.memberOf[steve] = view.id
        val channel = bvc.add("Theirs", "Bob")

        linker.resync()

        assertNotNull(table.bySvc(view.id))
        assertEquals(LinkEntry.Origin.BVC, table.byBvc(channel)!!.origin)
    }

    @Test
    fun `resync puts each member where their own side says`() {
        val (view, channel) = linkedSvcGroup()
        svc.memberOf[steve] = view.id
        bvc.channels[channel] = bvc.channels[channel]!!.copy(players = listOf("Bob"))

        linker.resync()

        assertTrue("join:Steve:$channel" in bvc.calls)
        assertEquals(view.id, svc.memberOf[bob])
    }

    @Test
    fun `reset_drain_triggers_resync`() {
        val view = svc.add("Raid")
        svc.memberOf[steve] = view.id

        bvcEvents(reset = true)

        assertNotNull(table.bySvc(view.id))
    }

    // A reset drain after a server restart is empty; the next poll must start from zero rather
    // than ask from a sequence number the server will never reach again.
    @Test
    fun `an empty reset drain starts the next poll from zero`() {
        bvcEvents(event(GroupLinkEventKind.Join, "C9", "Bob"))
        bvcEvents(reset = true)

        linker.pollBvc()

        assertEquals(0L, bvc.drainedAfter.last())
    }

    @Test
    fun `polls ask after the last seen event`() {
        bvcEvents(event(GroupLinkEventKind.Join, "C9", "Bob"), event(GroupLinkEventKind.Leave, "C9", "Bob"))

        linker.pollBvc()

        assertEquals(seq, bvc.drainedAfter.last())
    }

    @Test
    fun `groups_disabled_unlinks_everything`() {
        linkedSvcGroup()
        svc.enabled = false

        linker.resync()

        assertTrue(table.all().isEmpty())
    }

    // A lease stands in for Java members, who hold no BVC client. A group with none ages out
    // through BVC's reaper as before, and its Delete takes the SVC side with it.
    @Test
    fun `lease_renews_every_linked_channel_with_a_java_member`() {
        val (view, first) = linkedSvcGroup()
        svc.memberOf[steve] = view.id
        val second = bvc.add("Other", "Bob")
        bvcEvents(event(GroupLinkEventKind.Create, second, "Bob"))

        linker.renewLeases()

        assertEquals(listOf(first), bvc.leased.last())
    }

    // A pass the server thread did not answer must change nothing: it cannot tell "no groups"
    // from "no answer", and acting on the first would close or duplicate every linked group.
    @Test
    fun `an unanswered svc pass changes nothing`() {
        val (view, channel) = linkedSvcGroup()
        val bvcChannel = bvc.add("Theirs", "Bob")
        bvcEvents(event(GroupLinkEventKind.Create, bvcChannel, "Bob"))
        val linkedBefore = table.all().toSet()
        val bvcCallsBefore = bvc.calls.size
        svc.unavailable = true

        linker.resync()

        assertEquals(linkedBefore, table.all().toSet())
        assertEquals(bvcCallsBefore, bvc.calls.size)
        svc.unavailable = false
        assertTrue(svc.groups.containsKey(view.id))
        assertNotNull(table.byBvc(channel))
    }

    // A join that did not answer is not a join another plugin refused.
    @Test
    fun `an unanswered set group records no refusal`() {
        val (view, channel) = linkedSvcGroup()
        svc.unavailable = true

        bvcEvents(event(GroupLinkEventKind.Join, channel, "Bob"))

        assertFalse(refusals.isRefused(bob, view.id))
    }

    // The create ran on the server thread after the bridge stopped waiting. The next pass finds
    // the group by name and links it, instead of making a second one.
    @Test
    fun `a timed out create is linked by name on the next pass`() {
        matchFirstPassAway()
        val channel = bvc.add("Raid", "Bob")
        svc.createLandsButTimesOut = true
        bvcEvents(event(GroupLinkEventKind.Create, channel, "Bob"))
        assertNull(table.byBvc(channel))
        svc.createLandsButTimesOut = false

        linker.resync()

        assertEquals(1, svc.groups.values.count { it.name == "Raid" })
        assertNotNull(table.byBvc(channel))
    }

    // Name matching must not pair a private SVC group with a BVC channel whose speech is heard
    // nearby: the BVC flag cannot be changed after the fact.
    @Test
    fun `name matching needs the members only flags to agree`() {
        val view = svc.add("Raid", SvcGroupType.NORMAL)
        svc.memberOf[steve] = view.id
        val open = bvc.add("Raid", "Bob")

        linker.resync()

        assertTrue(table.bySvc(view.id)!!.bvcChannel != open)
    }

    // The link table lives in memory, so a restarted bridge sees a NORMAL group's BVC side with
    // no SVC group. Re-creating it as OPEN would let any Java player in and nearby players hear it.
    @Test
    fun `a members only channel with no svc group is closed`() {
        val channel = bvc.add("Raid", "Bob", membersOnly = true)

        linker.resync()

        assertTrue("close:$channel" in bvc.calls)
        assertTrue(svc.calls.none { it.startsWith("create:") })
    }

    // SVC owns an SVC-origin group's existence. Losing the BVC side (a BVC restart, a lapsed
    // lease) gives it a new BVC side rather than ending the Java players' group.
    @Test
    fun `a lost bvc side of an svc group is recreated`() {
        val (view, channel) = linkedSvcGroup(SvcGroupType.NORMAL)
        svc.memberOf[steve] = view.id
        bvc.channels.remove(channel)

        linker.resync()

        val entry = table.bySvc(view.id)!!
        assertTrue(entry.bvcChannel != channel)
        assertTrue(entry.membersOnly)
        assertTrue(svc.groups.containsKey(view.id))
        assertTrue("join:Steve:${entry.bvcChannel}" in bvc.calls)
    }

    @Test
    fun `a bvc delete of an svc group recreates its bvc side`() {
        val (view, channel) = linkedSvcGroup()
        svc.memberOf[steve] = view.id
        bvc.channels.remove(channel)

        bvcEvents(event(GroupLinkEventKind.Delete, channel, name = null))

        assertTrue(svc.groups.containsKey(view.id))
        val entry = table.bySvc(view.id)!!
        assertTrue(entry.bvcChannel != channel)
        assertTrue("join:Steve:${entry.bvcChannel}" in bvc.calls)
    }

    // Resync builds the starting state; replaying the feed's old events on top of it would move
    // players through groups that ended long ago.
    @Test
    fun `start skips the feed history`() {
        bvc.nextDrain = GroupLinkDrainJson(
            listOf(event(GroupLinkEventKind.Create, "C77", "Bob", name = "Old")),
            reset = false,
            epoch = 1L,
        )

        linker.start()
        linker.pollBvc()

        assertTrue(svc.calls.none { it == "create:Old" })
        assertEquals(seq, bvc.drainedAfter.last())
    }

    // A restarted server numbers its events from one again; the bridge cannot trust its cursor.
    @Test
    fun `a new server epoch resyncs`() {
        bvcEvents(event(GroupLinkEventKind.Join, "C9", "Bob"), epoch = 1L)
        val view = svc.add("Raid")
        svc.memberOf[steve] = view.id

        bvcEvents(epoch = 2L)

        assertNotNull(table.bySvc(view.id))
    }

    // Pairs nothing, so the next resync is not the first pass.
    private fun matchFirstPassAway() {
        linker.resync()
    }
}
