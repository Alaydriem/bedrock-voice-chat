package com.alaydriem.bedrockvoicechat.svc.link

import de.maxhenkel.voicechat.api.VoicechatServerApi
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Test
import java.lang.reflect.Proxy
import java.util.UUID

class SvcApiGroupsTest {
    private val steve = UUID.randomUUID()

    // Answers every SVC call with null: Steve is online but holds no voice connection, which is
    // enough for the snapshot to read his name.
    private val api = Proxy.newProxyInstance(
        VoicechatServerApi::class.java.classLoader,
        arrayOf(VoicechatServerApi::class.java),
    ) { _, _, _ -> null } as VoicechatServerApi

    // The linker names every SVC member to BVC through the snapshot. A snapshot that holds no
    // names links no group and joins no Java player to a BVC channel.
    @Test
    fun `refresh_reads_names_from_the_platform_lookup`() {
        val groups = SvcApiGroups(
            api = api,
            onlinePlayers = { listOf(steve) },
            onServerThread = { it.run() },
            platformNameOf = { if (it == steve) "Steve" else null },
        )

        groups.refresh()

        assertEquals("Steve", groups.nameOf(steve))
    }
}
