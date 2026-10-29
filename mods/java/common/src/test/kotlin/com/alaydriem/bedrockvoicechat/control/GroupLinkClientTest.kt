package com.alaydriem.bedrockvoicechat.control

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

class GroupLinkClientTest {

    // The same literal is deserialized on the Rust side in tests/http/group_link.rs, which is what
    // keeps the two hand-written encoders in step.
    @Test
    fun `create_linked_json_matches_rust_shape`() {
        assertEquals(
            """{"CreateLinked":{"creator":"Steve","name":"Raid","members_only":true}}""",
            GroupLinkClient.createLinkedJson("Steve", "Raid", true)
        )
    }

    @Test
    fun `close and lease json match the rust shape`() {
        assertEquals("""{"Close":{"channel":"ABCD"}}""", GroupLinkClient.closeJson("ABCD"))
        assertEquals(
            """{"Lease":{"channels":["ABCD","EFGH"]}}""",
            GroupLinkClient.leaseJson(listOf("ABCD", "EFGH"))
        )
    }

    @Test
    fun `drain_json_parses_events_and_reset`() {
        val drain = GroupLinkClient.parseDrain(
            """{"events":[{"seq":7,"kind":"Join","channel":"ABCD","channel_name":"Raid","player":"Alex","members_only":true}],"reset":true,"epoch":4503599627370495}"""
        )!!

        assertTrue(drain.reset)
        assertEquals(4503599627370495L, drain.epoch)
        val event = drain.events.single()
        assertEquals(7L, event.seq)
        assertEquals(GroupLinkEventKind.Join, event.kind)
        assertEquals("ABCD", event.channel)
        assertEquals("Raid", event.channelName)
        assertEquals("Alex", event.player)
        assertTrue(event.membersOnly)
    }

    // A Delete names no player and its channel is already gone, so the optional fields arrive null.
    @Test
    fun `a delete event parses with its optional fields absent`() {
        val drain = GroupLinkClient.parseDrain(
            """{"events":[{"seq":1,"kind":"Delete","channel":"ABCD","channel_name":null,"player":null,"members_only":false}],"reset":false,"epoch":1}"""
        )!!

        val event = drain.events.single()
        assertEquals(GroupLinkEventKind.Delete, event.kind)
        assertNull(event.player)
        assertFalse(drain.reset)
    }

    @Test
    fun `an undecodable drain is null rather than empty`() {
        assertNull(GroupLinkClient.parseDrain("not json"))
    }

    @Test
    fun `channels json parses players and members only`() {
        val channels = GroupLinkClient.parseChannels(
            """[{"id":"ABCD","name":"Raid","players":["Steve","Alex"],"members_only":true}]"""
        )!!

        val channel = channels.single()
        assertEquals("ABCD", channel.id)
        assertEquals(listOf("Steve", "Alex"), channel.players)
        assertTrue(channel.membersOnly)
    }

    // CreateLinked answers with the new id as a JSON string; every other action answers null.
    @Test
    fun `an action reply carries the created id`() {
        assertEquals("ABCD", GroupLinkClient.parseCreatedId("\"ABCD\""))
        assertNull(GroupLinkClient.parseCreatedId("null"))
    }
}
