package com.alaydriem.bedrockvoicechat.svc

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

class PeeringEligibilityTest {

    @Test
    fun `a server that advertises a peer link is eligible`() {
        val eligibility = PeeringEligibility { "bvcpeerAAAA" }

        assertTrue(eligibility.isEligible())
        assertEquals("bvcpeerAAAA", eligibility.resolve())
    }

    @Test
    fun `a server with peering off is not eligible`() {
        val eligibility = PeeringEligibility { null }

        assertFalse(eligibility.isEligible())
        assertNull(eligibility.resolve())
    }

    // /api/config is polled by clients and the mod has no reason to add to that. The link
    // does not change while the server runs.
    @Test
    fun `the peer link is fetched once and reused`() {
        var calls = 0
        val eligibility = PeeringEligibility {
            calls += 1
            "bvcpeerAAAA"
        }

        eligibility.resolve()
        eligibility.resolve()

        assertEquals(1, calls)
    }

    // A server that was briefly unreachable must not be remembered as ineligible, or the
    // operator would have to restart the game server to pair.
    @Test
    fun `a failed fetch is not cached`() {
        var calls = 0
        val eligibility = PeeringEligibility {
            calls += 1
            if (calls == 1) null else "bvcpeerAAAA"
        }

        assertNull(eligibility.resolve())
        assertEquals("bvcpeerAAAA", eligibility.resolve())
    }
}
