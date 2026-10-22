package com.alaydriem.bedrockvoicechat.control

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Test

class ControlOutcomeTest {

    // The server answers a protected join with 202 when it handed the join to the player's
    // BVC app, and 409 when the player has no app session to hand it to.
    @Test
    fun `http statuses map to the protected join outcomes`() {
        assertEquals(ControlOutcome.FORWARDED, ControlOutcome.fromHttpStatus(202))
        assertEquals(ControlOutcome.NO_SESSION, ControlOutcome.fromHttpStatus(409))
        assertNull(ControlOutcome.fromHttpStatus(200))
        assertNull(ControlOutcome.fromHttpStatus(404))
    }

    @Test
    fun `ffi return codes map to the protected join outcomes`() {
        assertEquals(ControlOutcome.FORWARDED, ControlOutcome.fromFfiCode(1))
        assertEquals(ControlOutcome.NO_SESSION, ControlOutcome.fromFfiCode(-2))
        assertNull(ControlOutcome.fromFfiCode(0))
        assertNull(ControlOutcome.fromFfiCode(-1))
    }

    @Test
    fun `a protected join points the player at the app`() {
        assertEquals(
            "[BVC] Your code for NR4K is in the BVC app.",
            JoinReply.forResult(ControlSendResult(true, outcome = ControlOutcome.FORWARDED), "NR4K"),
        )
        assertEquals(
            "[BVC] Open the BVC app to join NR4K.",
            JoinReply.forResult(ControlSendResult(false, outcome = ControlOutcome.NO_SESSION), "NR4K"),
        )
        assertNull(JoinReply.forResult(ControlSendResult(true), "NR4K"))
    }
}
