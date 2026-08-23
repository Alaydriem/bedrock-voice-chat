package com.alaydriem.bedrockvoicechat.svc

import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

class SvcAvailabilityTest {

    // The API is on the test classpath, so the positive case is real rather than
    // mocked.
    @Test
    fun `svc is detected when its api is on the classpath`() {
        assertTrue(SvcAvailability().isAvailable)
    }

    // A server without SVC is the ordinary case, not a fault. It must resolve to
    // absent rather than throwing, because the caller's next move is to skip the
    // bridge and start normally.
    @Test
    fun `a missing api class reports absent rather than throwing`() {
        assertFalse(SvcAvailability("de.maxhenkel.voicechat.api.NotAThing").isAvailable)
    }

    // Resolution is cached, so the check on the hot path costs nothing after the
    // first call and cannot flip mid-run.
    @Test
    fun `repeated checks agree`() {
        val availability = SvcAvailability()

        assertTrue(availability.isAvailable)
        assertTrue(availability.isAvailable)
    }
}
