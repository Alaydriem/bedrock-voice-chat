package com.alaydriem.bedrockvoicechat.audio

/**
 * Runs work on the server's main (tick) thread. Both methods may be called from any thread.
 */
interface MainThreadScheduler {
    fun runNow(task: () -> Unit)

    fun runLater(ticks: Long, task: () -> Unit): ScheduledTask
}
