package com.alaydriem.bedrockvoicechat.paper.chat

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Test

class PaperChatListenerTest {
    @Test
    fun `the console form carries no leading slash`() {
        assertEquals("hello world", PaperChatListener.sayArgument("say hello world"))
    }

    @Test
    fun `the player form does`() {
        assertEquals("hello world", PaperChatListener.sayArgument("/say hello world"))
    }

    @Test
    fun `the namespaced alias counts`() {
        assertEquals("hello", PaperChatListener.sayArgument("/minecraft:say hello"))
    }

    @Test
    fun `case does not matter`() {
        assertEquals("hello", PaperChatListener.sayArgument("SAY hello"))
    }

    @Test
    fun `a command that merely begins with those letters is not a say`() {
        assertNull(PaperChatListener.sayArgument("sayonara everyone"))
    }

    @Test
    fun `a say with no argument relays nothing`() {
        assertNull(PaperChatListener.sayArgument("say"))
        assertNull(PaperChatListener.sayArgument("/say   "))
    }

    @Test
    fun `an unrelated command relays nothing`() {
        assertNull(PaperChatListener.sayArgument("/gamemode creative"))
    }
}
