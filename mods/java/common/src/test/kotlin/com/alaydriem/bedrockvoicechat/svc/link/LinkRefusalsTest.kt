package com.alaydriem.bedrockvoicechat.svc.link

import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import java.util.UUID

class LinkRefusalsTest {
    private val player = UUID.randomUUID()
    private val group = UUID.randomUUID()

    // Another plugin cancelled the join; resync must not retry it on every pass.
    @Test
    fun `refusal_holds_until_cleared`() {
        val refusals = LinkRefusals()
        refusals.refuse(player, group)

        assertTrue(refusals.isRefused(player, group))
        assertFalse(refusals.isRefused(player, UUID.randomUUID()))

        refusals.clear(player)

        assertFalse(refusals.isRefused(player, group))
    }
}
