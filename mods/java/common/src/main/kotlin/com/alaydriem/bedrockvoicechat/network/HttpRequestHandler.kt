package com.alaydriem.bedrockvoicechat.network

import com.alaydriem.bedrockvoicechat.dto.Payload
import com.google.gson.Gson
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
            .header("X-MC-Access-Token", accessToken)

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
