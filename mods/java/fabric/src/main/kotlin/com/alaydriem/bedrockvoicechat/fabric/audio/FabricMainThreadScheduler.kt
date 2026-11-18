package com.alaydriem.bedrockvoicechat.fabric.audio

import com.alaydriem.bedrockvoicechat.audio.MainThreadScheduler
import com.alaydriem.bedrockvoicechat.audio.ScheduledTask
import org.slf4j.LoggerFactory
import java.util.concurrent.ConcurrentLinkedQueue

/**
 * Queues work onto the server tick. Fabric has no scheduler of its own, so nothing runs
 * until the owner calls [tick] once per server tick, on the server thread.
 */
class FabricMainThreadScheduler : MainThreadScheduler {

    private class DelayedTask(val dueTick: Long, val task: () -> Unit) : ScheduledTask {
        @Volatile
        var cancelled = false

        override fun cancel() {
            cancelled = true
        }
    }

    private val immediate = ConcurrentLinkedQueue<() -> Unit>()
    private val delayed = ConcurrentLinkedQueue<DelayedTask>()

    @Volatile
    private var currentTick = 0L

    override fun runNow(task: () -> Unit) {
        immediate.add(task)
    }

    override fun runLater(ticks: Long, task: () -> Unit): ScheduledTask =
        DelayedTask(currentTick + ticks, task).also { delayed.add(it) }

    /**
     * Advances one tick: runs everything queued by [runNow], then every delayed task that
     * has come due. A task queued while this runs waits for the next tick.
     */
    fun tick() {
        currentTick++

        repeat(immediate.size) {
            val task = immediate.poll() ?: return@repeat
            runIsolated(task)
        }

        val due = delayed.filter { it.cancelled || it.dueTick <= currentTick }
        delayed.removeAll(due.toSet())
        for (task in due) {
            if (!task.cancelled) runIsolated(task.task)
        }
    }

    // One failing task must not stop the tick or the tasks queued behind it.
    private fun runIsolated(task: () -> Unit) {
        try {
            task()
        } catch (e: Exception) {
            LOGGER.warn("A scheduled server task failed", e)
        }
    }

    private companion object {
        private val LOGGER = LoggerFactory.getLogger("Bedrock Voice Chat")
    }
}
