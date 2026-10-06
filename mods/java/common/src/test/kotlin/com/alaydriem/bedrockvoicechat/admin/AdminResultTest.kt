package com.alaydriem.bedrockvoicechat.admin

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Test

class AdminResultTest {
    @Test
    fun `zero is applied`() {
        assertEquals(AdminResult.APPLIED, AdminResult.fromCode(0))
    }

    @Test
    fun `one is nothing to revoke`() {
        assertEquals(AdminResult.NOTHING_TO_REVOKE, AdminResult.fromCode(1))
    }

    @Test
    fun `two is a gamertag that is not a player`() {
        assertEquals(AdminResult.PLAYER_NOT_FOUND, AdminResult.fromCode(2))
    }

    // An export that grows a new code must not be read as success by an older mod.
    @Test
    fun `any other code is a failure`() {
        assertEquals(AdminResult.FAILED, AdminResult.fromCode(-1))
        assertEquals(AdminResult.FAILED, AdminResult.fromCode(3))
    }
}
