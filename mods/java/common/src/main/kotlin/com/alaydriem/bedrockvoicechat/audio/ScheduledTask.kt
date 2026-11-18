package com.alaydriem.bedrockvoicechat.audio

/**
 * A task queued on a [MainThreadScheduler]. Cancelling a task that already ran does nothing.
 */
interface ScheduledTask {
    fun cancel()
}
