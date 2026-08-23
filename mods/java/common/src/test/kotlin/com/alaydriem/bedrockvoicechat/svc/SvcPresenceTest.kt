package com.alaydriem.bedrockvoicechat.svc

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import java.util.UUID

class SvcPresenceTest {

    private val steve = TestBodies.of("Steve")
    private val alex = TestBodies.of("Alex")

    private class Marks {
        val applied: MutableList<Pair<UUID, Boolean>> = mutableListOf()

        fun record(): (UUID, Boolean) -> Unit = { player, connected ->
            applied.add(player to connected)
        }

        fun last(player: UUID): Boolean? =
            applied.lastOrNull { it.first == player }?.second
    }

    private fun presence(
        online: List<UUID>,
        live: Set<UUID>,
        marks: Marks
    ) = SvcPresence(
        onlinePlayers = { online },
        hasLiveBvcClient = { live.contains(it) },
        setConnected = marks.record()
    )

    // The whole point: a player on the BVC app wore SVC's disconnected mark for
    // everyone standing next to them, while both sides could hear each other.
    @Test
    fun `a player on a BVC client is marked connected`() {
        val marks = Marks()

        presence(listOf(steve), setOf(steve), marks).reconcile()

        assertEquals(true, marks.last(steve))
    }

    // Withdrawn as well as applied. Marking only on the way in leaves somebody who
    // closed their BVC app shown as reachable for the rest of the session.
    @Test
    fun `a player with no BVC client is marked disconnected`() {
        val marks = Marks()

        presence(listOf(steve), emptySet(), marks).reconcile()

        assertEquals(false, marks.last(steve))
    }

    // The mark is per player, not a state of the server. Two people standing
    // together, one on the app and one not, must not be given the same answer.
    @Test
    fun `each player is answered for separately`() {
        val marks = Marks()

        presence(listOf(steve, alex), setOf(steve), marks).reconcile()

        assertEquals(true, marks.last(steve))
        assertEquals(false, marks.last(alex))
    }

    // A player who has left is not ours to mark. SVC answers for the living, and
    // reaching for somebody who has gone is how a sweep starts throwing.
    @Test
    fun `a player who is not online is not touched`() {
        val marks = Marks()

        presence(listOf(steve), setOf(steve, alex), marks).reconcile()

        assertTrue(marks.applied.none { it.first == alex })
    }

    // SVC resets the mark whenever a player's real state changes, so the sweep runs
    // on a timer and must be safe to repeat. Both directions are idempotent on SVC's
    // side, which is what makes repeating it cheap.
    @Test
    fun `sweeping twice reapplies rather than drifting`() {
        val marks = Marks()
        val presence = presence(listOf(steve), setOf(steve), marks)

        presence.reconcile()
        presence.reconcile()

        assertEquals(2, marks.applied.size)
        assertEquals(true, marks.last(steve))
    }
}
