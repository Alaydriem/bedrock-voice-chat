package com.alaydriem.bedrockvoicechat.chat

import com.google.gson.Gson
import com.google.gson.JsonSyntaxException
import org.slf4j.LoggerFactory
import java.net.URI
import java.net.http.HttpClient
import java.net.http.WebSocket
import java.util.concurrent.CompletableFuture
import java.util.concurrent.CompletionStage
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import kotlin.random.Random

/**
 * The mod's chat channel to the BVC server — one socket, both directions.
 *
 * Text frames of JSON tagged on `t`, matching `common::structs::chat::ChatFrame`. `hello` is
 * always first and names the world; nothing after it carries one.
 *
 * [send] is injected rather than called directly so the framing can be exercised without a
 * live socket. That seam is why the tests need no network.
 */
class ChatChannel(
    private val serverUrl: String,
    private val accessToken: String,
    private val worldUuid: String,
    private val worldName: String,
    /**
     * Every world id this chat room spans.
     *
     * Paper and Fabric mint one per dimension while chat is server-wide, so a room is several
     * ids. Empty means the room is exactly [worldUuid].
     */
    private val worlds: List<String> = emptyList(),
    private val onSay: (String, String) -> Unit,
    private val send: (String) -> Unit
) {
    companion object {
        private val LOGGER = LoggerFactory.getLogger("BVC Chat")
        private val GSON = Gson()
        private const val BACKOFF_MIN_MS = 2_000L
        private const val BACKOFF_MAX_MS = 60_000L
    }

    private data class Frame(val t: String?, val author: String?, val text: String?)

    private val retries = Executors.newSingleThreadScheduledExecutor { r ->
        Thread(r, "bvc-chat-retry").apply { isDaemon = true }
    }

    private var socket: WebSocket? = null
    private var backoff = BACKOFF_MIN_MS

    @Volatile
    private var stopped = false

    /** Sends `hello`. Called once the socket is open. */
    fun onOpen() {
        backoff = BACKOFF_MIN_MS
        send(
            GSON.toJson(
                mapOf(
                    "t" to "hello",
                    "world" to worldUuid,
                    "world_name" to worldName,
                    "game" to "minecraft",
                    "worlds" to worlds
                )
            )
        )
    }

    /**
     * Reports a line a player typed in game.
     *
     * Nothing is queued while the socket is down. Replaying a gap drops stale lines into a
     * conversation that has moved on, and no message is ever persisted.
     */
    fun report(author: String, text: String) {
        send(GSON.toJson(mapOf("t" to "chat", "author" to author, "text" to text)))
    }

    /** Handles a frame from the server. Only `say` is meaningful in this direction. */
    fun receive(body: String) {
        val frame = try {
            GSON.fromJson(body, Frame::class.java)
        } catch (e: JsonSyntaxException) {
            LOGGER.warn("chat channel received an undecodable frame")
            return
        } ?: return

        if (frame.t != "say" || frame.author == null || frame.text == null) {
            return
        }
        onSay(frame.author, frame.text)
    }

    fun connect() {
        if (stopped) return

        val uri = URI.create(serverUrl.replaceFirst("http", "ws") + "/api/websocket/chat")

        HttpClient.newHttpClient()
            .newWebSocketBuilder()
            .header("X-MC-Access-Token", accessToken)
            .buildAsync(uri, Listener())
            .whenComplete { ws, error ->
                if (error != null) {
                    LOGGER.warn("chat channel connect failed: ${error.message}")
                    scheduleRetry()
                } else {
                    socket = ws
                    onOpen()
                    LOGGER.info("chat channel connected")
                }
            }
    }

    fun close() {
        stopped = true
        socket?.sendClose(WebSocket.NORMAL_CLOSURE, "shutdown")
        socket = null
        retries.shutdownNow()
    }

    /** The real send, once a socket exists. Wired in by the caller at construction. */
    fun sendOverSocket(body: String) {
        socket?.sendText(body, true)
    }

    private fun scheduleRetry() {
        if (stopped) return

        // Jitter is load-bearing: without it a BVC server restart has every world's mod redial
        // in lockstep, and they all fail together again.
        val jitter = Random.nextLong(BACKOFF_MIN_MS)
        val delay = minOf(backoff, BACKOFF_MAX_MS) + jitter
        backoff = minOf(backoff * 2, BACKOFF_MAX_MS)
        retries.schedule({ connect() }, delay, TimeUnit.MILLISECONDS)
    }

    private inner class Listener : WebSocket.Listener {
        // A frame can arrive split across several onText calls; only the last carries the end
        // marker, so partial frames are accumulated rather than parsed individually.
        private val buffer = StringBuilder()

        override fun onText(
            webSocket: WebSocket,
            data: CharSequence,
            last: Boolean
        ): CompletionStage<*>? {
            buffer.append(data)
            if (last) {
                receive(buffer.toString())
                buffer.setLength(0)
            }
            webSocket.request(1)
            return CompletableFuture.completedFuture(null)
        }

        override fun onClose(
            webSocket: WebSocket,
            statusCode: Int,
            reason: String
        ): CompletionStage<*>? {
            LOGGER.warn("chat channel closed: $statusCode $reason")
            socket = null
            scheduleRetry()
            return CompletableFuture.completedFuture(null)
        }

        override fun onError(webSocket: WebSocket, error: Throwable) {
            LOGGER.warn("chat channel error: ${error.message}")
            socket = null
            scheduleRetry()
        }
    }
}
