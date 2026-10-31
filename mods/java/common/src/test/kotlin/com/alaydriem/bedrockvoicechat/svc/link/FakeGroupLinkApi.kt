package com.alaydriem.bedrockvoicechat.svc.link

import com.alaydriem.bedrockvoicechat.control.GroupLinkApi
import com.alaydriem.bedrockvoicechat.control.GroupLinkDrainJson
import com.alaydriem.bedrockvoicechat.control.LinkedChannelJson

/** BVC's channel state in memory. Calls change state only; events are driven by the test. */
class FakeGroupLinkApi : GroupLinkApi {
    val channels = LinkedHashMap<String, LinkedChannelJson>()
    val calls = ArrayList<String>()
    val leased = ArrayList<List<String>>()
    var nextDrain: GroupLinkDrainJson? = null
    val drainedAfter = ArrayList<Long>()
    private var nextId = 1

    fun add(name: String, vararg players: String, membersOnly: Boolean = false): String {
        val id = "C${nextId++}"
        channels[id] = LinkedChannelJson(id, name, players.toList(), membersOnly)
        return id
    }

    override fun createLinked(creator: String, name: String, membersOnly: Boolean): String {
        calls.add("createLinked:$creator:$name:$membersOnly")
        return add(name, membersOnly = membersOnly)
    }

    override fun close(channel: String): Boolean {
        calls.add("close:$channel")
        return channels.remove(channel) != null
    }

    override fun lease(channels: List<String>): Boolean {
        leased.add(channels)
        return true
    }

    override fun join(player: String, channel: String) {
        calls.add("join:$player:$channel")
        channels.replaceAll { id, c ->
            when {
                id == channel -> c.copy(players = (c.players - player) + player)
                else -> c.copy(players = c.players - player)
            }
        }
    }

    override fun leave(player: String) {
        calls.add("leave:$player")
        channels.replaceAll { _, c -> c.copy(players = c.players - player) }
    }

    override fun drain(after: Long): GroupLinkDrainJson? {
        drainedAfter.add(after)
        return nextDrain.also { nextDrain = null }
    }

    override fun channels(): List<LinkedChannelJson> = channels.values.toList()
}
