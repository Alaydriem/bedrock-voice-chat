package com.alaydriem.bedrockvoicechat.chat

import com.alaydriem.bedrockvoicechat.native.ChatFfi
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.atomic.AtomicInteger

class FfiChatTransportTest {
    /**
     * Refuses registration until [readyAfter] attempts, which is what the embedded server does
     * while it is still connecting its database and building its QUIC manager.
     */
    private class FakeFfi(private val readyAfter: Int) : ChatFfi {
        val attempts = AtomicInteger()
        val reported = CopyOnWriteArrayList<String>()

        @Volatile
        var registered = false

        @Volatile
        var pending: String = "[]"

        override fun chatRegister(helloJson: String): Boolean {
            if (attempts.incrementAndGet() < readyAfter) {
                return false
            }
            registered = true
            return true
        }

        override fun chatReport(chatJson: String): Boolean {
            reported.add(chatJson)
            return true
        }

        override fun chatDrain(): String? {
            val body = pending
            pending = "[]"
            return body
        }

        override fun chatUnregister(): Boolean {
            registered = false
            return true
        }
    }

    private fun transportFor(ffi: ChatFfi, onSay: (String, String) -> Unit = { _, _ -> }) =
        FfiChatTransport(ffi, "world-1", "Test Server", listOf("world-1"), onSay)

    private fun awaitTrue(timeoutMs: Long = 5_000, condition: () -> Boolean): Boolean {
        val deadline = System.nanoTime() + timeoutMs * 1_000_000
        while (System.nanoTime() < deadline) {
            if (condition()) {
                return true
            }
            Thread.sleep(25)
        }
        return condition()
    }

    @Test
    fun `registration is retried until the embedded server is ready`() {
        val ffi = FakeFfi(readyAfter = 3)
        val transport = transportFor(ffi)

        try {
            transport.start()
            assertTrue(awaitTrue { ffi.registered }, "registration never succeeded")
            assertTrue(ffi.attempts.get() >= 3, "the refusals were not retried")
        } finally {
            transport.stop()
        }
    }

    @Test
    fun `nothing is reported while registration is still refused`() {
        val ffi = FakeFfi(readyAfter = Int.MAX_VALUE)
        val transport = transportFor(ffi)

        try {
            transport.start()
            assertTrue(awaitTrue { ffi.attempts.get() >= 2 }, "registration was never attempted")

            transport.report("Alaydriem", "hello")
            transport.event("Alaydriem joined the game")

            assertEquals(emptyList<String>(), ffi.reported.toList())
        } finally {
            transport.stop()
        }
    }

    @Test
    fun `a say frame reaches the broadcaster once registered`() {
        val said = CopyOnWriteArrayList<Pair<String, String>>()
        val ffi = FakeFfi(readyAfter = 1)
        ffi.pending = """[{"t":"say","author":"Alaydriem","text":"from the app"}]"""
        val transport = transportFor(ffi) { author, text -> said.add(author to text) }

        try {
            transport.start()
            assertTrue(awaitTrue { said.isNotEmpty() }, "the say frame never arrived")
            assertEquals("Alaydriem" to "from the app", said.first())
        } finally {
            transport.stop()
        }
    }
}
