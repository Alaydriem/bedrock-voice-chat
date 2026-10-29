package com.alaydriem.bedrockvoicechat.control

import com.alaydriem.bedrockvoicechat.network.HttpRequestHandler
import com.alaydriem.bedrockvoicechat.server.BvcServerManager
import com.google.gson.Gson
import com.google.gson.JsonSyntaxException
import org.slf4j.LoggerFactory

/**
 * Routes group link calls to the embedded server over FFI or to an external one over HTTP, the
 * way [ControlSender] routes control actions. Joins and leaves are ordinary control actions.
 *
 * The JSON is built by hand because this mod shares no types with Rust's `common`; the shapes are
 * pinned on both sides by contract tests.
 */
class GroupLinkClient(
    private val httpHandler: HttpRequestHandler?,
    private val embeddedServer: BvcServerManager?,
    private val control: ControlSender,
) : GroupLinkApi {
    companion object {
        private val logger = LoggerFactory.getLogger("BVC Group Link")
        private val GSON = Gson()

        fun createLinkedJson(creator: String, name: String, membersOnly: Boolean): String =
            GSON.toJson(
                mapOf(
                    "CreateLinked" to mapOf(
                        "creator" to creator,
                        "name" to name,
                        "members_only" to membersOnly,
                    )
                )
            )

        fun closeJson(channel: String): String = GSON.toJson(mapOf("Close" to mapOf("channel" to channel)))

        fun leaseJson(channels: List<String>): String =
            GSON.toJson(mapOf("Lease" to mapOf("channels" to channels)))

        fun parseDrain(json: String): GroupLinkDrainJson? =
            try {
                GSON.fromJson(json, GroupLinkDrainJson::class.java)
            } catch (e: JsonSyntaxException) {
                logger.warn("Undecodable group link drain: {}", e.message)
                null
            }

        fun parseChannels(json: String): List<LinkedChannelJson>? =
            try {
                GSON.fromJson(json, Array<LinkedChannelJson>::class.java)?.toList()
            } catch (e: JsonSyntaxException) {
                logger.warn("Undecodable group link channel list: {}", e.message)
                null
            }

        fun parseCreatedId(body: String): String? =
            try {
                GSON.fromJson(body, String::class.java)
            } catch (e: JsonSyntaxException) {
                null
            }
    }

    private val embedded: BvcServerManager?
        get() = embeddedServer?.takeIf { it.isRunning }

    override fun createLinked(creator: String, name: String, membersOnly: Boolean): String? =
        action(createLinkedJson(creator, name, membersOnly))?.groupCode

    override fun close(channel: String): Boolean = action(closeJson(channel))?.ok == true

    override fun lease(channels: List<String>): Boolean = action(leaseJson(channels))?.ok == true

    override fun join(player: String, channel: String) = control.send(ControlAction.JoinGroup(channel), player)

    override fun leave(player: String) = control.send(ControlAction.LeaveGroup, player)

    override fun drain(after: Long): GroupLinkDrainJson? {
        val embedded = embedded
        val body = when {
            embedded != null -> embedded.groupLinkDrain(after)
            httpHandler != null -> httpHandler.groupLinkEvents(after)
            else -> null
        } ?: return null
        return parseDrain(body)
    }

    override fun channels(): List<LinkedChannelJson>? {
        val embedded = embedded
        val body = when {
            embedded != null -> embedded.groupLinkChannels()
            httpHandler != null -> httpHandler.groupLinkChannels()
            else -> null
        } ?: return null
        return parseChannels(body)
    }

    private fun action(json: String): ControlSendResult? {
        val embedded = embedded
        return when {
            embedded != null -> embedded.groupLinkAction(json)
            httpHandler != null -> httpHandler.groupLinkAction(json)
            else -> {
                logger.warn("No group link transport available (neither embedded nor HTTP configured)")
                null
            }
        }
    }
}
