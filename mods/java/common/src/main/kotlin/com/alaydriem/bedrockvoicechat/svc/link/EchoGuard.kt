package com.alaydriem.bedrockvoicechat.svc.link

/**
 * Recognises the events the bridge's own changes cause. Every SVC call the bridge makes fires an
 * SVC event back into it, and every BVC action fans a channel event back to it; acting on those
 * would loop.
 *
 * Each expectation absorbs one matching event. One that never arrives expires after [EXPIRY_MS],
 * so it cannot swallow a player's identical change later.
 *
 * Keys: `svc:join:<uuid>:<group>`, `svc:leave:<uuid>`, `svc:remove:<group>`,
 * `bvc:join:<player>:<channel>`, `bvc:leave:<player>`, `bvc:delete:<channel>`. Creates need no key:
 * the link table already holds the id the create call returned.
 */
class EchoGuard(private val clock: () -> Long = System::currentTimeMillis) {
    companion object {
        const val EXPIRY_MS: Long = 10_000
    }

    private val pending = HashMap<String, ArrayDeque<Long>>()

    @Synchronized
    fun expect(key: String) {
        pending.getOrPut(key) { ArrayDeque() }.addLast(clock() + EXPIRY_MS)
    }

    @Synchronized
    fun consume(key: String): Boolean {
        val deadlines = pending[key] ?: return false
        val now = clock()
        while (deadlines.isNotEmpty() && deadlines.first() < now) {
            deadlines.removeFirst()
        }
        val matched = deadlines.removeFirstOrNull() != null
        if (deadlines.isEmpty()) {
            pending.remove(key)
        }
        return matched
    }
}
