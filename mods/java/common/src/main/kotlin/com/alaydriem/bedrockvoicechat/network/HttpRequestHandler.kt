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

    /**
     * Send player data payload asynchronously to the BVC server.
     */
    fun sendAsync(payload: Payload) {
        val jsonBody = GSON.toJson(payload)
        val url = "$serverUrl/api/position"

        val request = HttpRequest.newBuilder()
            .uri(URI.create(url))
            .timeout(Duration.ofSeconds(5))
            .header("Content-Type", "application/json")
            .header("X-MC-Access-Token", accessToken)
            .header("Accept", "application/json")
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
}
