package com.alaydriem.bedrockvoicechat.network

import com.alaydriem.bedrockvoicechat.dto.Payload
import com.google.gson.Gson
import com.google.gson.JsonObject
import org.slf4j.LoggerFactory
import java.net.URI
import java.net.http.HttpClient
import java.net.http.HttpRequest
import java.net.http.HttpResponse
import java.time.Duration

/**
 * Handles HTTP requests to the BVC server.
 */
class HttpRequestHandler(
    private val serverUrl: String,
    private val accessToken: String,
    private val httpClient: HttpClient = HttpClient.newBuilder()
        .connectTimeout(Duration.ofSeconds(5))
        .build()
) {
    companion object {
        private val GSON = Gson()
        private val LOGGER = LoggerFactory.getLogger("Bedrock Voice Chat")
    }

    private fun requestBuilder(url: String): HttpRequest.Builder =
        HttpRequest.newBuilder()
            .uri(URI.create(url))
            .timeout(Duration.ofSeconds(5))
            .header("Authorization", "Bearer $accessToken")

    private fun jsonRequestBuilder(url: String): HttpRequest.Builder =
        requestBuilder(url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")

    fun sendAsync(payload: Payload) {
        val jsonBody = GSON.toJson(payload)
        val request = jsonRequestBuilder("$serverUrl/api/position")
            .POST(HttpRequest.BodyPublishers.ofString(jsonBody))
            .build()

        httpClient.sendAsync(request, HttpResponse.BodyHandlers.ofString())
            .thenAccept { response ->
                if (response.statusCode() in 200..299) {
                    LOGGER.debug("BVC server responded: {}", response.statusCode())
                } else {
                    LOGGER.warn("BVC server returned error: {} - {}", response.statusCode(), response.body())
                }
            }
            .exceptionally { ex ->
                LOGGER.error("Failed to send to BVC server: {}", ex.message)
                null
            }
    }

    fun audioPlayAsync(playJson: String, callback: (String?) -> Unit) {
        val request = jsonRequestBuilder("$serverUrl/api/audio/event")
            .POST(HttpRequest.BodyPublishers.ofString(playJson))
            .build()

        httpClient.sendAsync(request, HttpResponse.BodyHandlers.ofString())
            .thenAccept { response ->
                if (response.statusCode() in 200..299) {
                    callback(response.body())
                } else {
                    LOGGER.warn("BVC server returned error: {} - {}", response.statusCode(), response.body())
                    callback(null)
                }
            }
            .exceptionally { ex ->
                LOGGER.error("Failed to start audio playback: {}", ex.message)
                callback(null)
                null
            }
    }

    /**
     * The identities holding a live voice connection to the BVC server.
     *
     * Synchronous and called from a background refresh rather than from the audio
     * path: the answer changes on connect and disconnect, not per frame.
     *
     * Null distinguishes "could not ask" from an empty list. They are not the same:
     * an empty list means suppress nobody, and a failed request means we do not know
     * — and a caller that conflated them would silently stop suppressing whenever
     * the server was briefly unreachable.
     */
    fun liveClients(): List<String>? {
        val request = requestBuilder("$serverUrl/api/clients/live")
            .header("Accept", "application/json")
            .GET()
            .build()

        return try {
            val response = httpClient.send(request, HttpResponse.BodyHandlers.ofString())
            if (response.statusCode() !in 200..299) {
                LOGGER.debug("BVC server returned {} for live clients", response.statusCode())
                return null
            }
            GSON.fromJson(response.body(), Array<String>::class.java).toList()
        } catch (e: Exception) {
            LOGGER.debug("Failed to read live clients: {}", e.toString())
            null
        }
    }

    /**
     * This server's peer link, or null when it does not peer.
     *
     * `/api/config` is unauthenticated, so this works before the mod has an access token
     * and on a server that has never seen this mod. The Authorization header the shared
     * builder adds is ignored by that route.
     */
    fun serverPeerLink(): String? {
        val request = requestBuilder("$serverUrl/api/config")
            .header("Accept", "application/json")
            .GET()
            .build()

        return try {
            val response = httpClient.send(request, HttpResponse.BodyHandlers.ofString())
            if (response.statusCode() !in 200..299) {
                LOGGER.debug("BVC server returned {} for config", response.statusCode())
                return null
            }
            val body = GSON.fromJson(response.body(), JsonObject::class.java)
            body.get("peer_link")?.takeIf { !it.isJsonNull }?.asString
        } catch (e: Exception) {
            LOGGER.debug("Failed to read the server config: {}", e.toString())
            null
        }
    }

    /**
     * POSTs a host capability report.
     *
     * Fire and forget, and never retried. This is a measurement, so a host that
     * cannot deliver it should not spend anything trying — and a failure to report
     * is itself uninteresting, because the interesting failures are the ones the
     * report already describes.
     */
    fun hostCapabilityAsync(reportJson: String) {
        val request = jsonRequestBuilder("$serverUrl/api/telemetry/host-capability")
            .POST(HttpRequest.BodyPublishers.ofString(reportJson))
            .build()

        httpClient.sendAsync(request, HttpResponse.BodyHandlers.ofString())
            .thenAccept { response ->
                if (response.statusCode() !in 200..299) {
                    LOGGER.debug(
                        "BVC server returned {} on host capability report",
                        response.statusCode()
                    )
                }
            }
            .exceptionally { ex ->
                LOGGER.debug("Failed to send host capability report: {}", ex.message)
                null
            }
    }

    /**
     * POSTs a ClientAction; [onResult] (optional) receives the outcome and — for a
     * successful CreateGroup — the new group's share code, invoked from the HTTP
     * client's executor thread.
     */
    fun controlAsync(clientActionJson: String, onResult: ((ok: Boolean, groupCode: String?) -> Unit)? = null) {
        val request = jsonRequestBuilder("$serverUrl/api/control")
            .POST(HttpRequest.BodyPublishers.ofString(clientActionJson))
            .build()

        httpClient.sendAsync(request, HttpResponse.BodyHandlers.ofString())
            .thenAccept { response ->
                val ok = response.statusCode() in 200..299
                if (!ok) {
                    LOGGER.warn("BVC server returned error on control: {} - {}", response.statusCode(), response.body())
                }
                onResult?.invoke(ok, if (ok) parseGroupCode(response.body()) else null)
            }
            .exceptionally { ex ->
                LOGGER.error("Failed to send control action: {}", ex.message)
                onResult?.invoke(false, null)
                null
            }
    }

    // The control route replies with the new group's share code (a bare JSON
    // string) for CreateGroup and `null` for everything else.
    private fun parseGroupCode(body: String): String? = try {
        val element = com.google.gson.JsonParser.parseString(body)
        if (element.isJsonPrimitive && element.asJsonPrimitive.isString) element.asString else null
    } catch (_: Exception) {
        null
    }

    fun audioStopAsync(eventId: String) {
        val request = requestBuilder("$serverUrl/api/audio/event/$eventId")
            .DELETE()
            .build()

        httpClient.sendAsync(request, HttpResponse.BodyHandlers.ofString())
            .thenAccept { response ->
                if (response.statusCode() !in 200..299) {
                    LOGGER.warn("BVC server returned error stopping audio: {} - {}", response.statusCode(), response.body())
                }
            }
            .exceptionally { ex ->
                LOGGER.error("Failed to stop audio playback: {}", ex.message)
                null
            }
    }
}
