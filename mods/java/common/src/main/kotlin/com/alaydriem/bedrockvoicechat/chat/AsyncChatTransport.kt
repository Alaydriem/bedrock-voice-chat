package com.alaydriem.bedrockvoicechat.chat

import org.slf4j.LoggerFactory
import java.util.concurrent.ArrayBlockingQueue
import java.util.concurrent.RejectedExecutionHandler
import java.util.concurrent.ThreadPoolExecutor
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean

/**
 * Runs a [ChatTransport] on its own thread.
 *
 * Everything that feeds chat reaches it from the game's main thread: Paper's death, join, quit
 * and broadcast events, and Fabric's chat event. Both transports can block there — the FFI takes
 * a lock the position tick also holds, and a socket send waits on the network. A main thread
 * that pauses stops answering movement packets, and a player who was only falling is then
 * disconnected for flying.
 *
 * One thread rather than a pool, because chat is ordered and a pool would not keep it so.
 */
class AsyncChatTransport(private val delegate: ChatTransport) : ChatTransport {
    companion object {
        private val LOGGER = LoggerFactory.getLogger("BVC Chat")

        /** Deep enough for a burst, shallow enough that a stalled transport drops rather than grows. */
        private const val QUEUE_DEPTH = 256

        private const val SHUTDOWN_GRACE_MS = 1_000L
    }

    private val dropping = AtomicBoolean(false)

    private val worker = ThreadPoolExecutor(
        1,
        1,
        0L,
        TimeUnit.MILLISECONDS,
        ArrayBlockingQueue(QUEUE_DEPTH),
        { r -> Thread(r, "bvc-chat").apply { isDaemon = true } },
        RejectedExecutionHandler { _, _ ->
            // Once per backlog, not once per line: a full queue means the transport is wedged,
            // and a message per dropped line would bury the one that says so.
            if (dropping.compareAndSet(false, true)) {
                LOGGER.warn("chat backlog is full; lines are being dropped")
            }
        }
    )

    /**
     * Registration runs here too, so it keeps its place in front of every line it must precede.
     */
    override fun start() = submit { delegate.start() }

    override fun report(author: String, text: String) = submit { delegate.report(author, text) }

    override fun event(text: String) = submit { delegate.event(text) }

    /**
     * Waits briefly, because the delegate releases the world's registration and the caller tears
     * the server down immediately afterwards.
     */
    override fun stop() {
        submit { delegate.stop() }
        worker.shutdown()
        if (!worker.awaitTermination(SHUTDOWN_GRACE_MS, TimeUnit.MILLISECONDS)) {
            worker.shutdownNow()
        }
    }

    private fun submit(work: () -> Unit) {
        worker.execute {
            try {
                work()
            } catch (e: Throwable) {
                LOGGER.warn("chat transport failed: ${e.message}")
            } finally {
                if (worker.queue.isEmpty()) {
                    dropping.set(false)
                }
            }
        }
    }
}
