package com.alaydriem.bedrockvoicechat.svc.link

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import java.util.UUID

class SvcLinkTableTest {
    private val entry = LinkEntry(UUID.randomUUID(), "ABCD", membersOnly = true, origin = LinkEntry.Origin.SVC)

    @Test
    fun `table_finds_both_directions`() {
        val table = SvcLinkTable()
        table.put(entry)

        assertEquals(entry, table.bySvc(entry.svcGroup))
        assertEquals(entry, table.byBvc("ABCD"))
    }

    @Test
    fun `remove_clears_both_keys`() {
        val table = SvcLinkTable()
        table.put(entry)

        table.remove(entry)

        assertNull(table.bySvc(entry.svcGroup))
        assertNull(table.byBvc("ABCD"))
        assertTrue(table.all().isEmpty())
    }

    // Re-linking a BVC channel to a re-created SVC group must not leave the old group pointing at it.
    @Test
    fun `relinking a channel drops the old svc group`() {
        val table = SvcLinkTable()
        table.put(entry)
        val recreated = entry.copy(svcGroup = UUID.randomUUID())

        table.put(recreated)

        assertNull(table.bySvc(entry.svcGroup))
        assertEquals(recreated, table.byBvc("ABCD"))
        assertEquals(1, table.all().size)
    }
}
