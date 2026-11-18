package com.alaydriem.bedrockvoicechat.svc.link

import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

class EchoGuardTest {
    private var now = 0L
    private val guard = EchoGuard { now }

    // The bridge's own change comes back as an event exactly once; a second, identical event is a
    // player's and must be acted on.
    @Test
    fun `echo_is_consumed_once`() {
        guard.expect("svc:join:p:g")

        assertTrue(guard.consume("svc:join:p:g"))
        assertFalse(guard.consume("svc:join:p:g"))
    }

    @Test
    fun `unexpected_event_is_not_consumed`() {
        guard.expect("svc:join:p:g")

        assertFalse(guard.consume("svc:join:p:other"))
    }

    // An echo that never arrives must not swallow a real event much later.
    @Test
    fun `expectation_expires`() {
        guard.expect("bvc:leave:Steve")
        now += EchoGuard.EXPIRY_MS + 1

        assertFalse(guard.consume("bvc:leave:Steve"))
    }

    // Two of the bridge's own calls with the same key produce two echoes.
    @Test
    fun `two expectations absorb two echoes`() {
        guard.expect("bvc:join:Steve:A")
        guard.expect("bvc:join:Steve:A")

        assertTrue(guard.consume("bvc:join:Steve:A"))
        assertTrue(guard.consume("bvc:join:Steve:A"))
        assertFalse(guard.consume("bvc:join:Steve:A"))
    }
}
