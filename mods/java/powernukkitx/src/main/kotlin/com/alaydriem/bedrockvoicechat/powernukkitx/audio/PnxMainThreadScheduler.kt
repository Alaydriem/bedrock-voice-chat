package com.alaydriem.bedrockvoicechat.powernukkitx.audio

import com.alaydriem.bedrockvoicechat.audio.MainThreadScheduler
import com.alaydriem.bedrockvoicechat.audio.ScheduledTask
import org.powernukkitx.plugin.Plugin

class PnxMainThreadScheduler(private val plugin: Plugin) : MainThreadScheduler {

    // PowerNukkitX throws when a disabled plugin schedules a task, and a late HTTP
    // response can arrive after onDisable. Nothing is left on the main thread to run it for.
    override fun runNow(task: () -> Unit) {
        if (!plugin.isEnabled()) return
        plugin.getServer().getScheduler().scheduleTask(plugin, Runnable { task() })
    }

    override fun runLater(ticks: Long, task: () -> Unit): ScheduledTask {
        if (!plugin.isEnabled()) return NOTHING_SCHEDULED
        val delay = ticks.coerceIn(0L, Int.MAX_VALUE.toLong()).toInt()
        val handler = plugin.getServer().getScheduler().scheduleDelayedTask(plugin, Runnable { task() }, delay)
        return object : ScheduledTask {
            override fun cancel() = handler.cancel()
        }
    }

    private companion object {
        val NOTHING_SCHEDULED = object : ScheduledTask {
            override fun cancel() {}
        }
    }
}
