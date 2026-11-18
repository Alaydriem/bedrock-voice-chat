package com.alaydriem.bedrockvoicechat.paper.audio

import com.alaydriem.bedrockvoicechat.audio.MainThreadScheduler
import com.alaydriem.bedrockvoicechat.audio.ScheduledTask
import org.bukkit.plugin.java.JavaPlugin

class BukkitMainThreadScheduler(private val plugin: JavaPlugin) : MainThreadScheduler {

    // Bukkit refuses to schedule for a disabled plugin, and a late HTTP response can
    // arrive after onDisable. Nothing is left on the main thread to run it for.
    override fun runNow(task: () -> Unit) {
        if (!plugin.isEnabled) return
        plugin.server.scheduler.runTask(plugin, Runnable { task() })
    }

    override fun runLater(ticks: Long, task: () -> Unit): ScheduledTask {
        val bukkitTask = plugin.server.scheduler.runTaskLater(plugin, Runnable { task() }, ticks)
        return object : ScheduledTask {
            override fun cancel() = bukkitTask.cancel()
        }
    }
}
