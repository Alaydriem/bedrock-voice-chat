package com.alaydriem.bedrockvoicechat.chat

import com.alaydriem.bedrockvoicechat.native.BvcNative
import com.alaydriem.bedrockvoicechat.native.ChatFfi
import com.google.gson.Gson
import com.google.gson.JsonSyntaxException
import org.slf4j.LoggerFactory
import java.util.concurrent.Executors
import java.util.concurrent.ScheduledExecutorService
import java.util.concurrent.TimeUnit

/**
 * Chat for embedded mode, over FFI.
 *
 * The BVC server shares this process, so there is nothing to dial. Registering through the FFI
 * puts this transport in the same `ChatSocketRegistry` a WebSocket would occupy, which is what
 * lets fan-out, availability and the app's world picker work identically in both modes.
 *
 * Server to mod is a poll rather than a push, because the FFI has no callback mechanism. At
 * [POLL_INTERVAL_MS] the added latency is invisible for chat.
 *
 * Registration is retried on that same poll. The embedded server returns a handle as soon as it
 * is created, but its chat hub does not exist until startup has connected the database, run
 * migrations and built the QUIC manager — seconds later. A single attempt at mod startup is
 * always refused, which is why this cannot be a one-shot.
 */
class FfiChatTransport(
    private val server: ChatFfi,
    private val worldUuid: String,
    private val worldName: String,
    private val worlds: List<String>,
    private val onSay: (String, String) -> Unit
) : ChatTransport {
    companion object {
        private val LOGGER = LoggerFactory.getLogger("BVC Chat")
        private val GSON = Gson()

        /** Fast enough that nobody notices, slow enough to cost nothing. */
        private const val POLL_INTERVAL_MS = 250L

        /** Ten seconds of refusals is longer than any normal startup. */
        private const val WARN_AFTER_ATTEMPTS = 40

        /** A minute between repeats, so a server that never starts says so without a storm. */
        private const val WARN_EVERY_ATTEMPTS = 240
    }

    private data class Frame(val t: String?, val author: String?, val text: String?)

    private var poller: ScheduledExecutorService? = null

    @Volatile
    private var registered = false

    private var attempts = 0

    override fun start() {
        val executor = Executors.newSingleThreadScheduledExecutor { r ->
            Thread(r, "bvc-chat-ffi").apply { isDaemon = true }
        }
        executor.scheduleWithFixedDelay(
            { poll() },
            0,
            POLL_INTERVAL_MS,
            TimeUnit.MILLISECONDS
        )
        poller = executor
    }

    override fun report(author: String, text: String) {
        if (!registered) {
            return
        }
        server.chatReport(GSON.toJson(mapOf("t" to "chat", "author" to author, "text" to text)))
    }

    override fun event(text: String) {
        if (!registered) {
            return
        }
        server.chatReport(GSON.toJson(mapOf("t" to "event", "text" to text)))
    }

    override fun stop() {
        poller?.shutdownNow()
        poller = null
        if (registered) {
            registered = false
            server.chatUnregister()
        }
    }

    private fun poll() {
        if (registered) {
            drain()
        } else {
            register()
        }
    }

    private fun register() {
        attempts++

        val hello = GSON.toJson(
            mapOf(
                "t" to "hello",
                "world" to worldUuid,
                "world_name" to worldName,
                "game" to "minecraft",
                "worlds" to worlds
            )
        )

        if (server.chatRegister(hello)) {
            registered = true
            LOGGER.info("chat relay started over FFI")
            return
        }

        // Early refusals are the server still starting, and saying so every quarter second
        // would bury the message that matters.
        if (attempts == WARN_AFTER_ATTEMPTS || attempts % WARN_EVERY_ATTEMPTS == 0) {
            LOGGER.warn(
                "chat relay still not registered after {} attempts: {}",
                attempts,
                BvcNative.getLastError()
            )
        }
    }

    private fun drain() {
        val body = server.chatDrain() ?: return
        if (body == "[]") {
            return
        }

        val frames = try {
            GSON.fromJson(body, Array<Frame>::class.java)
        } catch (e: JsonSyntaxException) {
            LOGGER.warn("undecodable chat frames from the server")
            return
        } ?: return

        for (frame in frames) {
            if (frame.t != "say" || frame.author == null || frame.text == null) {
                continue
            }
            onSay(frame.author, frame.text)
        }
    }
}
