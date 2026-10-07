package com.alaydriem.bedrockvoicechat.admin

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Test

class AdminResultTest {
    // An export that grows a new code must not be read as success by an older mod.
    @Test
    fun `an unknown code is a failure`() {
        assertEquals(AdminResult.FAILED, AdminResult.fromCode(-1))
        assertEquals(AdminResult.FAILED, AdminResult.fromCode(3))
    }
}
