package com.alaydriem.bedrockvoicechat.svc.link

import org.slf4j.LoggerFactory

/**
 * One pass of the link thread's work, with the clock and the queued events handed in.
 *
 * Each pass refreshes the SVC snapshot, builds the starting state or resyncs when one is due,
 * drains BVC's channel events, then runs the SVC events that were waiting. The parts fail apart:
 * one that throws does not take the others with it.
 *
 * BVC's events are not read until [SvcGroupLinker.start] has succeeded. Reading them first would
 * replay the feed's history onto a state no resync has built.
 */
class SvcLinkLoop(
    private val linker: SvcGroupLinker,
    private val refresh: () -> Unit,
    private val afterRefresh: () -> Unit,
    private val resyncIntervalMs: Long = RESYNC_INTERVAL_MS,
) {
    private var started = false
    private var lastResync = 0L

    /** Never sleeps and starts no thread. Called from the link thread alone. */
    fun tick(now: Long, events: List<() -> Unit>) {
        part("the SVC snapshot") {
            refresh()
            afterRefresh()
        }
        part("the group resync") {
            if (!started) {
                if (linker.start()) {
                    started = true
                    lastResync = now
                    linker.renewLeases()
                }
            } else if (now - lastResync >= resyncIntervalMs) {
                lastResync = now
                linker.resync()
                linker.renewLeases()
            }
        }
        if (started) {
            part("BVC's channel events") { linker.pollBvc() }
        }
        for (event in events) {
            part("an SVC group event") { event() }
        }
    }

    private inline fun part(label: String, block: () -> Unit) {
        try {
            block()
        } catch (e: InterruptedException) {
            Thread.currentThread().interrupt()
        } catch (e: Exception) {
            logger.warn("Group link pass failed at {}: {}", label, e.toString())
        }
    }

    companion object {
        // Catches an event either side missed. Also the lease renewal interval: three renewals
        // fit in the server's 30 second lease.
        const val RESYNC_INTERVAL_MS: Long = 10_000

        private val logger = LoggerFactory.getLogger("BVC Group Link")
    }
}
