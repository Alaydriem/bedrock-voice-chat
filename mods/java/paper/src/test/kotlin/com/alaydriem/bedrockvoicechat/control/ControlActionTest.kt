package com.alaydriem.bedrockvoicechat.control

import com.google.gson.JsonParser
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Test

/**
 * Golden-vector test for the Java control encoder. The JSON [ControlAction] produces
 * must match common's `ClientActionType` serde (contract-tested on the Rust side in
 * common/tests/structs/control/client_action_json.rs). These vectors are the same
 * shapes the BDS `ControlCodec` and the Rust decoder agree on; a drift here desyncs
 * the standalone mod from the server.
 */
class ControlActionTest {

    private fun assertJson(expected: String, action: ControlAction) {
        val actual = action.toClientActionJson("Alice")
        assertEquals(
            JsonParser.parseString(expected),
            JsonParser.parseString(actual),
            "toClientActionJson mismatch for $action -> $actual",
        )
    }

    @Test
    fun mute_serializes_as_bool_tagged_variant() {
        assertJson("""{"id":"Alice","game":"minecraft","action":{"SetMuted":true}}""", ControlAction.Mute(true))
    }

    @Test
    fun deafen_serializes_as_bool_tagged_variant() {
        assertJson("""{"id":"Alice","game":"minecraft","action":{"SetDeafened":false}}""", ControlAction.Deafen(false))
    }

    @Test
    fun record_serializes_as_bool_tagged_variant() {
        assertJson("""{"id":"Alice","game":"minecraft","action":{"SetRecording":true}}""", ControlAction.Record(true))
    }

    @Test
    fun volume_percent_becomes_fraction() {
        assertJson(
            """{"id":"Alice","game":"minecraft","action":{"SetVolume":{"target":"Steve","volume":0.7}}}""",
            ControlAction.Volume("Steve", 70),
        )
    }

    @Test
    fun hear_flag_inverts_to_muted() {
        assertJson(
            """{"id":"Alice","game":"minecraft","action":{"SetHeard":{"target":"Steve","muted":true}}}""",
            ControlAction.Hear("Steve", false),
        )
        assertJson(
            """{"id":"Alice","game":"minecraft","action":{"SetHeard":{"target":"Steve","muted":false}}}""",
            ControlAction.Hear("Steve", true),
        )
    }

    @Test
    fun create_group_is_a_bare_string() {
        assertJson("""{"id":"Alice","game":"minecraft","action":"CreateGroup"}""", ControlAction.CreateGroup)
    }

    @Test
    fun join_group_carries_channel() {
        assertJson(
            """{"id":"Alice","game":"minecraft","action":{"JoinGroup":{"channel":"abc123"}}}""",
            ControlAction.JoinGroup("abc123"),
        )
    }

    @Test
    fun leave_group_is_a_bare_string() {
        assertJson("""{"id":"Alice","game":"minecraft","action":"LeaveGroup"}""", ControlAction.LeaveGroup)
    }
}
