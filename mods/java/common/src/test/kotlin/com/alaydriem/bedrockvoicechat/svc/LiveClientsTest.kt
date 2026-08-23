package com.alaydriem.bedrockvoicechat.svc

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

class LiveClientsTest {

    // Embedded answers per call over FFI, which is a lookup in a live index rather
    // than a request, so there is nothing to cache and nothing to go stale.
    @Test
    fun `a direct source is asked every time`() {
        var asks = 0
        val clients = LiveClients.direct { name -> asks += 1; name == "DualStack" }

        assertTrue(clients.isLive("DualStack"))
        assertFalse(clients.isLive("JavaOnly"))
        assertEquals(2, asks)
    }

    // External answers over HTTP, so it is refreshed on a schedule and read from a
    // snapshot. Asking per frame would put a network round trip on the audio path.
    @Test
    fun `a polled source reads the last snapshot`() {
        val clients = LiveClients.polled { listOf("DualStack") }
        clients.refresh()

        assertTrue(clients.isLive("DualStack"))
        assertFalse(clients.isLive("JavaOnly"))
    }

    // Before the first refresh nothing is known, and the honest answer is to
    // suppress nobody: injecting to a player who also runs the BVC client is a
    // doubled voice, while suppressing one who does not is silence. Doubling is the
    // better failure.
    @Test
    fun `nobody is live before the first refresh`() {
        assertFalse(LiveClients.polled { listOf("DualStack") }.isLive("DualStack"))
    }

    // A failed fetch keeps the previous snapshot rather than clearing it. Treating
    // "could not ask" as "nobody is connected" would resume double-audio for every
    // dual-stack player whenever the server blinked.
    @Test
    fun `a failed refresh keeps what was already known`() {
        var answer: List<String>? = listOf("DualStack")
        val clients = LiveClients.polled { answer }

        clients.refresh()
        answer = null
        clients.refresh()

        assertTrue(clients.isLive("DualStack"))
    }

    @Test
    fun `a refresh replaces the previous snapshot`() {
        var answer = listOf("DualStack")
        val clients = LiveClients.polled { answer }

        clients.refresh()
        answer = listOf("SomeoneElse")
        clients.refresh()

        assertFalse(clients.isLive("DualStack"))
        assertTrue(clients.isLive("SomeoneElse"))
    }
}
