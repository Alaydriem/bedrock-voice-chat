package com.alaydriem.bedrockvoicechat.svc.link

import java.util.UUID
import java.util.concurrent.ConcurrentHashMap

/**
 * Joins another plugin cancelled. A refused pair is not retried, by an event or by resync, until
 * the player's BVC membership changes.
 */
class LinkRefusals {
    private val refused = ConcurrentHashMap<UUID, MutableSet<UUID>>()

    fun refuse(player: UUID, group: UUID) {
        refused.computeIfAbsent(player) { ConcurrentHashMap.newKeySet() }.add(group)
    }

    fun isRefused(player: UUID, group: UUID): Boolean = refused[player]?.contains(group) == true

    fun clear(player: UUID) {
        refused.remove(player)
    }
}
