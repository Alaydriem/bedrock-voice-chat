package com.alaydriem.bedrockvoicechat.svc.link

import java.util.UUID
import java.util.concurrent.ConcurrentHashMap

/**
 * SVC group <-> BVC channel. Rebuilt from both sides when the bridge starts, so it is held in
 * memory only. Thread-safe: SVC events arrive on the server thread and BVC events on the link
 * thread.
 */
class SvcLinkTable {
    private val bySvc = ConcurrentHashMap<UUID, LinkEntry>()
    private val byBvc = ConcurrentHashMap<String, LinkEntry>()

    @Synchronized
    fun put(entry: LinkEntry) {
        byBvc[entry.bvcChannel]?.let { bySvc.remove(it.svcGroup) }
        bySvc[entry.svcGroup]?.let { byBvc.remove(it.bvcChannel) }
        bySvc[entry.svcGroup] = entry
        byBvc[entry.bvcChannel] = entry
    }

    fun bySvc(id: UUID): LinkEntry? = bySvc[id]

    fun byBvc(id: String): LinkEntry? = byBvc[id]

    @Synchronized
    fun remove(entry: LinkEntry) {
        bySvc.remove(entry.svcGroup)
        byBvc.remove(entry.bvcChannel)
    }

    fun all(): List<LinkEntry> = bySvc.values.toList()
}
