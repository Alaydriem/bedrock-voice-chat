package com.alaydriem.bedrockvoicechat.powernukkitx.chat

import com.alaydriem.bedrockvoicechat.chat.ChatTransport
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Test
import org.mockito.kotlin.mock
import org.mockito.kotlin.verify
import org.powernukkitx.Server

class PnxChatListenerTest {

    @Test
    fun colour_codes_and_line_breaks_are_removed() {
        assertEquals("[Server] stop now x", PnxChatListener.sanitize("§c[Server]§r stop now\nx"))
    }

    @Test
    fun an_app_line_cannot_forge_a_second_line_or_a_colour() {
        val server = mock<Server>()
        val listener = PnxChatListener(mock<ChatTransport>(), mock<PnxMessageTranslator>(), server)

        listener.say("§4Admin", "hi\n§e<Admin> trust me")

        verify(server).broadcastMessage("<Admin> hi <Admin> trust me")
    }
}
