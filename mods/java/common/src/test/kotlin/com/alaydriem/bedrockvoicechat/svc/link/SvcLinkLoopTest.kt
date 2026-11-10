package com.alaydriem.bedrockvoicechat.svc.link

import com.alaydriem.bedrockvoicechat.control.GroupLinkDrainJson
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import java.util.UUID

class SvcLinkLoopTest {
    // Steve plays Java with the SVC mod and is in one SVC group, so the first resync links it
    // and every lease renewal has a channel to name.
    private val steve = UUID.randomUUID()
    private val svc = FakeSvcGroups()
    private val bvc = FakeGroupLinkApi()
    private val linker = SvcGroupLinker(
        svc = svc,
        bvc = bvc,
        table = SvcLinkTable(),
        sides = PlayerSides(hasLiveBvcClient = { false }, hasSvcInstalled = { it == steve }),
        echo = EchoGuard(),
        refusals = LinkRefusals(),
        actorIdOf = { if (it == steve) "Steve" else null },
        playerOf = { if (it == "Steve") steve else null },
    )
    private val loop = SvcLinkLoop(linker, refresh = {}, afterRefresh = {})

    private fun quietDrain(latest: Long = 0) =
        GroupLinkDrainJson(emptyList(), reset = false, epoch = 1L, latest = latest)

    private fun steveInAGroup() {
        svc.memberOf[steve] = svc.add("Raid").id
    }

    @Test
    fun `the first tick builds the starting state and leases it`() {
        steveInAGroup()
        bvc.nextDrain = quietDrain()

        loop.tick(0, emptyList())

        assertTrue(bvc.calls.any { it.startsWith("createLinked:Steve:Raid") })
        assertEquals(1, bvc.leased.size)
    }

    // An external BVC server can be down when the Java server starts. Until it answers there is
    // no cursor, and a poll from zero would replay everything the feed holds.
    @Test
    fun `a start that cannot reach bvc is tried again and its history is not read`() {
        bvc.nextDrain = null
        loop.tick(0, emptyList())
        assertEquals(listOf(0L), bvc.drainedAfter)

        bvc.nextDrain = quietDrain(latest = 3)
        loop.tick(250, emptyList())
        loop.tick(500, emptyList())

        assertEquals(3L, bvc.drainedAfter.last())
    }

    @Test
    fun `resync and lease run again after ten seconds and not before`() {
        steveInAGroup()
        bvc.nextDrain = quietDrain()
        loop.tick(0, emptyList())

        loop.tick(9_999, emptyList())
        assertEquals(1, bvc.leased.size)

        loop.tick(10_000, emptyList())
        assertEquals(2, bvc.leased.size)
    }

    @Test
    fun `a pass that throws still runs the queued events`() {
        var ran = false
        val failing = SvcLinkLoop(linker, refresh = { error("boom") }, afterRefresh = {})

        failing.tick(0, listOf({ ran = true }))

        assertTrue(ran)
    }
}
