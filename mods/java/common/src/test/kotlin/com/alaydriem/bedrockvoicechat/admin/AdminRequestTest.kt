package com.alaydriem.bedrockvoicechat.admin

import com.alaydriem.bedrockvoicechat.dto.GameType
import com.google.gson.Gson
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Test

// The JSON must match common's `AdminPermissionRequest` serde; the client e2e `admin`
// scenario drives the export with this shape.
class AdminRequestTest {
    @Test
    fun `serializes to the AdminPermissionRequest wire shape`() {
        val json = Gson().toJson(AdminRequest("Some Name", GameType.MINECRAFT, AdminAction.DENY))

        assertEquals("""{"gamertag":"Some Name","game":"minecraft","action":"deny"}""", json)
    }
}
