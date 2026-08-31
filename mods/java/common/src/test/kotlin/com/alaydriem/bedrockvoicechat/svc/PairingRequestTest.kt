package com.alaydriem.bedrockvoicechat.svc

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

class PairingRequestTest {

    @Test
    fun `a server that does not peer is reported rather than dialled`() {
        var dialled = false
        val request = PairingRequest(PeeringEligibility { null }, {}) { _, _ ->
            dialled = true
            PairingOutcome.Paired
        }

        assertEquals(PairingOutcome.NotEligible, request.submit("K7M49QTR"))
        assertFalse(dialled)
    }

    @Test
    fun `a successful redemption reports paired`() {
        val request = PairingRequest(PeeringEligibility { "bvcpeerAAAA" }, {}) { link, code ->
            assertEquals("bvcpeerAAAA", link)
            assertEquals("K7M49QTR", code)
            PairingOutcome.Paired
        }

        assertEquals(PairingOutcome.Paired, request.submit("K7M49QTR"))
    }

    // The bridge's connect thread runs once, when Simple Voice Chat hands over its API,
    // and ends there when nothing has granted this node yet. Redemption is the event that
    // changes that answer, so it is also what has to reopen the session — otherwise the
    // operator is paired, told voice is active, and has neither audio nor a connected mark
    // until the server is restarted.
    @Test
    fun `a successful redemption opens the bridge`() {
        var opened = 0
        val request = PairingRequest(PeeringEligibility { "bvcpeerAAAA" }, { opened += 1 }) { _, _ ->
            PairingOutcome.Paired
        }

        request.submit("K7M49QTR")

        assertEquals(1, opened)
    }

    // The server refusing a node it already knows is what an operator running the command a
    // second time sees, and their reason for running it again is usually that voice is not
    // working. Leaving the bridge closed there answers the wrong question.
    @Test
    fun `a bridge the server already knows is opened too`() {
        var opened = false
        val request = PairingRequest(PeeringEligibility { "bvcpeerAAAA" }, { opened = true }) { _, _ ->
            PairingOutcome.AlreadyPaired
        }

        request.submit("K7M49QTR")

        assertTrue(opened)
    }

    @Test
    fun `a refused code leaves the bridge closed`() {
        var opened = false
        val request = PairingRequest(PeeringEligibility { "bvcpeerAAAA" }, { opened = true }) { _, _ ->
            PairingOutcome.WrongCode
        }

        request.submit("K7M49QTR")

        assertFalse(opened)
    }

    // The operator types what is on their screen. Normalising here means a code carrying the
    // dash the CLI printed, or in the case the terminal used, still reaches the server as
    // the value it minted.
    @Test
    fun `a typed code is normalised before it is sent`() {
        val request = PairingRequest(PeeringEligibility { "bvcpeerAAAA" }, {}) { _, code ->
            assertEquals("K7M49QTR", code)
            PairingOutcome.Paired
        }

        request.submit(" k7m4-9qtr ")
    }

    // The server folds these the same way. A divergence means a code that mints on one side
    // and fails on the other, which reads to an operator as a wrong code.
    @Test
    fun `the excluded letters fold the same way the server folds them`() {
        assertEquals("1K7M49QTR", PairingRequest.normalize("IK7M49QTR"))
        assertEquals("1K7M49QTR", PairingRequest.normalize("lK7M49QTR"))
        assertEquals("0K7M49QTR", PairingRequest.normalize("OK7M49QTR"))
    }
}
