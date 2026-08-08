package com.alaydriem.bedrockvoicechat.chat

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

class ChatChannelTest {
    private fun channel(
        sent: MutableList<String> = mutableListOf(),
        said: MutableList<Pair<String, String>> = mutableListOf()
    ) = ChatChannel(
        serverUrl = "https://bvc.example",
        accessToken = "tok",
        worldUuid = "w1",
        worldName = "Survival",
        onSay = { author, text -> said.add(author to text) },
        send = { sent.add(it) },
        worlds = listOf("w1", "nether")
    )

    @Test
    fun `hello is the first frame and names the world`() {
        val sent = mutableListOf<String>()
        channel(sent = sent).onOpen()

        assertTrue(sent.first().contains("\"t\":\"hello\""), sent.first())
        assertTrue(sent.first().contains("\"world\":\"w1\""), sent.first())
        assertTrue(sent.first().contains("Survival"), sent.first())
    }

    @Test
    fun `a reported line is a chat frame carrying the author`() {
        val sent = mutableListOf<String>()
        val c = channel(sent = sent)

        c.onOpen()
        c.report("Petra", "anyone got spare iron")

        assertTrue(sent.last().contains("\"t\":\"chat\""), sent.last())
        assertTrue(sent.last().contains("Petra"), sent.last())
        assertTrue(sent.last().contains("anyone got spare iron"), sent.last())
    }

    @Test
    fun `a say frame is handed to the listener`() {
        val said = mutableListOf<Pair<String, String>>()

        channel(said = said).receive("""{"t":"say","author":"Alaydriem","text":"from the app"}""")

        assertEquals("Alaydriem" to "from the app", said.single())
    }

    // The server never sends `chat` in this direction, and honouring one would let a
    // compromised server put words in a player's mouth locally.
    @Test
    fun `a chat frame from the server is ignored`() {
        val said = mutableListOf<Pair<String, String>>()

        channel(said = said).receive("""{"t":"chat","author":"Petra","text":"hello"}""")

        assertTrue(said.isEmpty())
    }

    @Test
    fun `an undecodable frame is ignored rather than thrown`() {
        val said = mutableListOf<Pair<String, String>>()

        channel(said = said).receive("not json")

        assertTrue(said.isEmpty())
    }

    @Test
    fun `a say frame missing its text is ignored`() {
        val said = mutableListOf<Pair<String, String>>()

        channel(said = said).receive("""{"t":"say","author":"Alaydriem"}""")

        assertTrue(said.isEmpty())
    }

    // Paper and Fabric mint a world id per dimension while chat is server-wide, so the room
    // declares every id it spans. Without this a line typed in the overworld never reaches
    // somebody standing in the nether.
    @Test
    fun `hello declares every world the room spans`() {
        val sent = mutableListOf<String>()
        channel(sent = sent).onOpen()

        assertTrue(sent.first().contains("\"worlds\""), sent.first())
        assertTrue(sent.first().contains("nether"), sent.first())
    }
}
