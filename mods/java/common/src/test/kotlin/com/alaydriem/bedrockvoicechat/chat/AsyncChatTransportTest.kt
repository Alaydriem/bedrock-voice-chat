package com.alaydriem.bedrockvoicechat.chat

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNotEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

class AsyncChatTransportTest {
    private class Recording(private val done: CountDownLatch) : ChatTransport {
        val threads = CopyOnWriteArrayList<String>()
        val lines = CopyOnWriteArrayList<String>()

        override fun start() {}

        override fun report(author: String, text: String) {
            threads.add(Thread.currentThread().name)
            lines.add(text)
            done.countDown()
        }

        override fun event(text: String) {
            threads.add(Thread.currentThread().name)
            lines.add(text)
            done.countDown()
        }

        override fun stop() {}
    }

    @Test
    fun `a reported line reaches the transport off the calling thread`() {
        val done = CountDownLatch(1)
        val delegate = Recording(done)

        AsyncChatTransport(delegate).report("Alaydriem", "hello")

        assertTrue(done.await(5, TimeUnit.SECONDS), "the line never reached the transport")
        assertNotEquals(Thread.currentThread().name, delegate.threads.first())
    }

    @Test
    fun `an event reaches the transport off the calling thread`() {
        val done = CountDownLatch(1)
        val delegate = Recording(done)

        AsyncChatTransport(delegate).event("Alaydriem was killed by a creeper")

        assertTrue(done.await(5, TimeUnit.SECONDS), "the event never reached the transport")
        assertNotEquals(Thread.currentThread().name, delegate.threads.first())
    }

    @Test
    fun `lines keep the order they were reported in`() {
        val count = 50
        val done = CountDownLatch(count)
        val delegate = Recording(done)
        val transport = AsyncChatTransport(delegate)

        for (i in 0 until count) {
            transport.report("Alaydriem", i.toString())
        }

        assertTrue(done.await(5, TimeUnit.SECONDS), "not every line arrived")
        assertEquals((0 until count).map { it.toString() }, delegate.lines.toList())
    }
}
