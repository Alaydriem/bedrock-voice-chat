package com.alaydriem.bedrockvoicechat.svc

import com.alaydriem.bedrockvoicechat.svc.link.SvcGroupType
import com.alaydriem.bedrockvoicechat.svc.link.SvcGroupView
import de.maxhenkel.voicechat.api.Position
import de.maxhenkel.voicechat.api.ServerLevel
import de.maxhenkel.voicechat.api.ServerPlayer
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import java.util.UUID

class SvcSuppressionTest {

    private class FakePlayer(private val id: UUID) : ServerPlayer {
        override fun getUuid(): UUID = id
        override fun getEntity(): Any = this
        override fun getPosition(): Position = throw UnsupportedOperationException()
        override fun getPlayer(): Any = this
        override fun getServerLevel(): ServerLevel = throw UnsupportedOperationException()
    }

    private val dualStack = UUID.randomUUID()
    private val javaOnly = UUID.randomUUID()

    // Populations are disjoint by design, but a Java player can run both. Injecting
    // to them as well as to their BVC client delivers every remote speaker twice.
    @Test
    fun `a listener running the bvc client is excluded`() {
        val speakers = SvcSpeakers { it == dualStack }

        assertFalse(speakers.filter().test(FakePlayer(dualStack)))
    }

    @Test
    fun `an ordinary svc listener is included`() {
        val speakers = SvcSpeakers { false }

        assertTrue(speakers.filter().test(FakePlayer(javaOnly)))
    }

    // Decided locally, from state the mod already has. Nothing about this crosses
    // the wire: the sending side does not know who runs what here.
    @Test
    fun `the decision is made from the listener alone`() {
        var asked: UUID? = null
        val speakers = SvcSpeakers { id -> asked = id; false }

        speakers.filter().test(FakePlayer(javaOnly))

        assertEquals(javaOnly, asked)
    }

    // The predicate is re-evaluated per delivery, so a player who starts or stops
    // their BVC client mid-session is handled without reopening the channel.
    @Test
    fun `the filter reflects a change without being rebuilt`() {
        var running = false
        val filter = SvcSpeakers { running }.filter()

        assertTrue(filter.test(FakePlayer(dualStack)))
        running = true
        assertFalse(filter.test(FakePlayer(dualStack)))
    }

    private val group = UUID.randomUUID()

    private fun view(type: SvcGroupType) = SvcGroupView(group, "Raid", type, hasPassword = false, hidden = false)

    // A member of the speaker's linked group already hears them on a group channel; the proximity
    // copy would play them twice.
    @Test
    fun `group_member_is_excluded_from_proximity_copy`() {
        val speakers = SvcSpeakers(groupOf = { view(SvcGroupType.OPEN) }) { false }
        speakers.note("Alex", group)

        assertFalse(speakers.filter("Alex").test(FakePlayer(javaOnly)))
        assertTrue(speakers.filter("Bob").test(FakePlayer(javaOnly)))
    }

    // SVC keeps an ISOLATED group's members from hearing nearby non-members; the bridge's
    // proximity injection honours that for Java players.
    @Test
    fun `isolated_listener_does_not_hear_non_member`() {
        val speakers = SvcSpeakers(groupOf = { view(SvcGroupType.ISOLATED) }) { false }

        assertFalse(speakers.filter("Bob").test(FakePlayer(javaOnly)))
    }

    @Test
    fun `a normal group listener still hears nearby speakers`() {
        val speakers = SvcSpeakers(groupOf = { view(SvcGroupType.NORMAL) }) { false }

        assertTrue(speakers.filter("Bob").test(FakePlayer(javaOnly)))
    }

    // A speaker who leaves the linked group is heard nearby again.
    @Test
    fun `a cleared note restores the proximity copy`() {
        val speakers = SvcSpeakers(groupOf = { view(SvcGroupType.OPEN) }) { false }
        speakers.note("Alex", group)
        speakers.note("Alex", null)

        assertTrue(speakers.filter("Alex").test(FakePlayer(javaOnly)))
    }
}
