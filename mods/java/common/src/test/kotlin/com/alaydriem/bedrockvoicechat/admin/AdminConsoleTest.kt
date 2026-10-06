package com.alaydriem.bedrockvoicechat.admin

import com.alaydriem.bedrockvoicechat.api.ConfigProvider
import com.alaydriem.bedrockvoicechat.config.ModConfig
import com.alaydriem.bedrockvoicechat.server.BvcServerManager
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Test
import java.nio.file.Path

class AdminConsoleTest {
    private val bootstrapHint =
        "This server uses an external BVC server. Run \"bvc-server admin bootstrap -p Alice\" on that host."

    @Test
    fun `external mode points the operator at bootstrap`() {
        val console = AdminConsole { null }

        assertEquals(bootstrapHint, console.run(AdminAction.GRANT, "Alice"))
    }

    // A manager that never started holds no handle, which is the same situation as
    // external mode from the command's point of view.
    @Test
    fun `an embedded server that is not running points the operator at bootstrap`() {
        val provider = object : ConfigProvider {
            override fun load(): ModConfig = ModConfig()
            override fun createDefaultIfMissing() {}
            override fun getConfigDir(): Path? = null
        }
        val manager = BvcServerManager(ModConfig(), provider)
        val console = AdminConsole { manager }

        assertEquals(bootstrapHint, console.run(AdminAction.REVOKE, "Alice"))
    }

    @Test
    fun `a gamertag that is not a player is named as such`() {
        val target = object : AdminTarget {
            override fun admin(gamertag: String, action: AdminAction) = AdminResult.PLAYER_NOT_FOUND
            override fun lastError(): String? = null
        }
        val console = AdminConsole { target }

        assertEquals(
            "Mallory is not a BVC player; nothing was changed.",
            console.run(AdminAction.DENY, "Mallory")
        )
    }

    // A native library older than the mod has no bvc_admin symbol. JNA resolves it on the
    // first call and throws an Error, which would otherwise escape the command as a trace.
    @Test
    fun `a native library without the admin export is reported instead of thrown`() {
        val target = object : AdminTarget {
            override fun admin(gamertag: String, action: AdminAction): AdminResult =
                throw UnsatisfiedLinkError("Error looking up function 'bvc_admin'")
            override fun lastError(): String? = null
        }
        val console = AdminConsole { target }

        assertEquals(
            "The BVC native library predates /bvc admin. Update the plugin and restart the server.",
            console.run(AdminAction.GRANT, "Alice")
        )
    }
}
