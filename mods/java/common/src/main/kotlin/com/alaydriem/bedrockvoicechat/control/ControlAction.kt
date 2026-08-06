package com.alaydriem.bedrockvoicechat.control

import com.google.gson.Gson

/**
 * An in-game control action. This mod does not share types with the Rust `common`
 * crate; the JSON produced by [toClientActionJson] must match common's
 * `ClientActionType` serde (unit variants as bare strings, struct/tuple variants as
 * `{ Variant: ... }`), which is contract-tested on the Rust side.
 */
sealed class ControlAction {
    data class Mute(val on: Boolean) : ControlAction()
    data class Deafen(val on: Boolean) : ControlAction()
    data class Record(val on: Boolean) : ControlAction()
    data class Volume(val target: String, val value: Int) : ControlAction()
    data class Hear(val target: String, val on: Boolean) : ControlAction()
    object CreateGroup : ControlAction()
    data class JoinGroup(val channel: String) : ControlAction()
    object LeaveGroup : ControlAction()

    // Wire variant tags, matching common's `ClientActionType` serde exactly. Using an
    // enum (rather than string literals) makes a rename a compile error here instead
    // of a silent desync with the Rust decoder.
    private enum class Tag {
        SetMuted,
        SetDeafened,
        SetRecording,
        SetVolume,
        SetHeard,
        CreateGroup,
        JoinGroup,
        LeaveGroup,
    }

    private fun actionJson(): Any =
        when (this) {
            is Mute -> mapOf(Tag.SetMuted.name to on)
            is Deafen -> mapOf(Tag.SetDeafened.name to on)
            is Record -> mapOf(Tag.SetRecording.name to on)
            is Volume -> mapOf(Tag.SetVolume.name to mapOf("target" to target, "volume" to value / 100.0))
            is Hear -> mapOf(Tag.SetHeard.name to mapOf("target" to target, "muted" to !on))
            is CreateGroup -> Tag.CreateGroup.name
            is JoinGroup -> mapOf(Tag.JoinGroup.name to mapOf("channel" to channel))
            is LeaveGroup -> Tag.LeaveGroup.name
        }

    // `game` labels the actor the same way the /api/position body does, so the server
    // builds this player's canonical `game:gamertag` key from a declared value instead
    // of assuming one. A Java mod player is always Minecraft.
    fun toClientActionJson(actorId: String): String =
        GSON.toJson(mapOf("id" to actorId, "game" to "minecraft", "action" to actionJson()))

    companion object {
        private val GSON = Gson()
    }
}
