package com.alaydriem.bedrockvoicechat.admin

import com.alaydriem.bedrockvoicechat.api.ConfigProvider
import com.alaydriem.bedrockvoicechat.config.ModConfig
import com.alaydriem.bedrockvoicechat.dto.GameType
import com.alaydriem.bedrockvoicechat.server.BvcServerManager
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Test
import java.nio.file.Path

class AdminConsoleTest {
    private val bootstrapHint =
        "This server uses an external BVC server. Run \"bvc-server admin bootstrap -p Alice\" on that host."

    private class RecordingTarget(private val result: AdminResult) : AdminTarget {
        var received: AdminRequest? = null

        override fun admin(request: AdminRequest): AdminResult {
            received = request
            return result
        }

        override fun lastError(): String? = "boom"
    }

    @Test
    fun `the request carries the console's game, the action and the gamertag`() {
        val target = RecordingTarget(AdminResult.APPLIED)
        val console = AdminConsole(GameType.MINECRAFT) { target }

        assertEquals("Granted BVC admin to Alice.", console.run(AdminAction.GRANT, "Alice"))
        assertEquals(AdminRequest("Alice", GameType.MINECRAFT, AdminAction.GRANT), target.received)
    }

    @Test
    fun `a failure reports the native error`() {
        val console = AdminConsole(GameType.MINECRAFT) { RecordingTarget(AdminResult.FAILED) }

        assertEquals("BVC admin change failed: boom.", console.run(AdminAction.DENY, "Alice"))
    }

    @Test
    fun `external mode points the operator at bootstrap`() {
        val console = AdminConsole(GameType.MINECRAFT) { null }

        assertEquals(bootstrapHint, console.run(AdminAction.GRANT, "Alice"))
    }

    // A manager that never started holds no handle, which the command treats as external.
    @Test
    fun `an embedded server that is not running points the operator at bootstrap`() {
        val provider = object : ConfigProvider {
            override fun load(): ModConfig = ModConfig()
            override fun createDefaultIfMissing() {}
            override fun getConfigDir(): Path? = null
        }
        val manager = BvcServerManager(ModConfig(), provider)
        val console = AdminConsole(GameType.MINECRAFT) { manager }

        assertEquals(bootstrapHint, console.run(AdminAction.REVOKE, "Alice"))
    }

    @Test
    fun `a native library without the admin export is reported instead of thrown`() {
        val target = object : AdminTarget {
            override fun admin(request: AdminRequest): AdminResult =
                throw UnsatisfiedLinkError("Error looking up function 'bvc_admin'")
            override fun lastError(): String? = null
        }
        val console = AdminConsole(GameType.MINECRAFT) { target }

        assertEquals(
            "The BVC native library predates /bvc admin. Update the plugin and restart the server.",
            console.run(AdminAction.GRANT, "Alice")
        )
    }
}
