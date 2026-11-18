package com.alaydriem.bedrockvoicechat.audio

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

class JukeboxPlaybackServiceTest {

    private class FakePort : AudioEventPort {
        val played = mutableListOf<String>()
        val stopped = mutableListOf<String>()
        private val callbacks = mutableListOf<(String?) -> Unit>()

        override fun play(json: String, callback: (String?) -> Unit) {
            played.add(json)
            callbacks.add(callback)
        }

        override fun stop(eventId: String) {
            stopped.add(eventId)
        }

        fun respond(index: Int, response: String?) = callbacks[index](response)
    }

    private class FakeTask(val ticks: Long, val task: () -> Unit) : ScheduledTask {
        var cancelled = false

        override fun cancel() {
            cancelled = true
        }

        fun fire() {
            if (!cancelled) task()
        }
    }

    private class FakeScheduler : MainThreadScheduler {
        val later = mutableListOf<FakeTask>()
        val queued = mutableListOf<() -> Unit>()

        // Off: runNow runs at once. On: runNow queues, like a real main thread that
        // has stopped ticking, and nothing runs until runQueued.
        var deferNow = false

        override fun runNow(task: () -> Unit) {
            if (deferNow) queued.add(task) else task()
        }

        fun runQueued() {
            val tasks = queued.toList()
            queued.clear()
            tasks.forEach { it() }
        }

        override fun runLater(ticks: Long, task: () -> Unit): ScheduledTask =
            FakeTask(ticks, task).also { later.add(it) }
    }

    private val port = FakePort()
    private val scheduler = FakeScheduler()
    private val service = JukeboxPlaybackService(port, scheduler) { "relay-world" }
    private val started = mutableListOf<Long>()
    private val ended = mutableListOf<String>()
    private val key = service.locationKey("world-1", 10, 64, -5)

    init {
        service.onEnded { ended.add(it) }
    }

    private fun start() =
        service.startPlayback("audio-1", "overworld", 10.0, 64.0, -5.0, "world-1") { started.add(it) }

    @Test
    fun `success reports duration and schedules eject at duration in ticks`() {
        start()
        assertTrue(service.hasActivePlayback(key))

        port.respond(0, """{"event_id":"e1","duration_ms":1000}""")

        assertEquals(listOf(1000L), started)
        assertEquals(1, scheduler.later.size)
        assertEquals(20L, scheduler.later[0].ticks)
        assertTrue(service.hasActivePlayback(key))
    }

    @Test
    fun `the play request carries the position, world and relay world`() {
        start()

        val json = port.played.single()
        assertTrue(json.contains("\"audio_file_id\":\"audio-1\""))
        assertTrue(json.contains("\"world_uuid\":\"world-1\""))
        assertTrue(json.contains("\"relay_world_uuid\":\"relay-world\""))
        assertTrue(json.contains("\"dimension\":\"overworld\""))
    }

    @Test
    fun `auto eject stops the event and frees the location`() {
        start()
        port.respond(0, """{"event_id":"e1","duration_ms":1000}""")

        scheduler.later[0].fire()

        assertEquals(listOf("e1"), port.stopped)
        assertFalse(service.hasActivePlayback(key))
        assertEquals(listOf(key), ended)
    }

    @Test
    fun `a zero duration success ejects after one tick`() {
        start()
        port.respond(0, """{"event_id":"e1","duration_ms":0}""")

        assertEquals(listOf(0L), started)
        assertEquals(1L, scheduler.later.single().ticks)
    }

    @Test
    fun `failed request frees the location and reports zero`() {
        start()

        port.respond(0, null)

        assertEquals(listOf(0L), started)
        assertFalse(service.hasActivePlayback(key))
        assertTrue(scheduler.later.isEmpty())
    }

    @Test
    fun `unparseable response frees the location and reports zero`() {
        start()

        port.respond(0, "not json")

        assertEquals(listOf(0L), started)
        assertFalse(service.hasActivePlayback(key))
        assertTrue(scheduler.later.isEmpty())
    }

    @Test
    fun `a response without event id is treated as a failure`() {
        start()

        port.respond(0, """{"duration_ms":1000}""")

        assertEquals(listOf(0L), started)
        assertFalse(service.hasActivePlayback(key))
        assertTrue(scheduler.later.isEmpty())
    }

    @Test
    fun `stop cancels the pending eject`() {
        start()
        port.respond(0, """{"event_id":"e1","duration_ms":1000}""")

        service.stopPlayback(key)

        assertTrue(scheduler.later[0].cancelled)
        assertEquals(listOf("e1"), port.stopped)
        assertFalse(service.hasActivePlayback(key))
        assertTrue(ended.isEmpty())
    }

    @Test
    fun `shutdown stops every event`() {
        start()
        service.startPlayback("audio-2", "overworld", 1.0, 2.0, 3.0, "world-1") { started.add(it) }
        port.respond(0, """{"event_id":"e1","duration_ms":1000}""")
        port.respond(1, """{"event_id":"e2","duration_ms":2000}""")

        service.shutdown()

        assertEquals(setOf("e1", "e2"), port.stopped.toSet())
        assertTrue(scheduler.later.all { it.cancelled })
        assertFalse(service.hasActivePlayback(key))
        assertFalse(service.hasActivePlayback(service.locationKey("world-1", 1, 2, 3)))
    }

    @Test
    fun `a success arriving after stop stops that event and reports nothing`() {
        start()
        service.stopPlayback(key)

        port.respond(0, """{"event_id":"e1","duration_ms":1000}""")

        assertEquals(listOf("e1"), port.stopped)
        assertTrue(started.isEmpty())
        assertTrue(scheduler.later.isEmpty())
        assertFalse(service.hasActivePlayback(key))
    }

    @Test
    fun `a success arriving after shutdown stops that event and reports nothing`() {
        start()
        service.shutdown()

        port.respond(0, """{"event_id":"e1","duration_ms":1000}""")

        assertEquals(listOf("e1"), port.stopped)
        assertTrue(started.isEmpty())
    }

    @Test
    fun `a success arriving after shutdown is stopped without the main thread`() {
        scheduler.deferNow = true
        start()
        service.shutdown()

        port.respond(0, """{"event_id":"e1","duration_ms":1000}""")

        assertEquals(listOf("e1"), port.stopped)
        assertTrue(scheduler.queued.isEmpty())
        assertTrue(started.isEmpty())
    }

    @Test
    fun `a success queued for the main thread when shutdown runs is stopped by shutdown`() {
        scheduler.deferNow = true
        start()
        port.respond(0, """{"event_id":"e1","duration_ms":1000}""")

        service.shutdown()

        assertEquals(listOf("e1"), port.stopped)

        scheduler.runQueued()

        assertEquals(listOf("e1"), port.stopped)
        assertTrue(started.isEmpty())
        assertTrue(scheduler.later.isEmpty())
    }

    @Test
    fun `a queued success still starts normally when the main thread runs it`() {
        scheduler.deferNow = true
        start()
        port.respond(0, """{"event_id":"e1","duration_ms":1000}""")

        scheduler.runQueued()

        assertEquals(listOf(1000L), started)
        assertTrue(port.stopped.isEmpty())
        assertTrue(service.hasActivePlayback(key))
    }

    @Test
    fun `a late success for a replaced playback does not touch the new one`() {
        start()
        service.stopPlayback(key)
        start()

        port.respond(0, """{"event_id":"e1","duration_ms":1000}""")
        port.respond(1, """{"event_id":"e2","duration_ms":1000}""")

        assertEquals(listOf("e1"), port.stopped)
        assertEquals(listOf(1000L), started)
        assertTrue(service.hasActivePlayback(key))
    }
}
