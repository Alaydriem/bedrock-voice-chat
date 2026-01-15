package com.alaydriem.bedrockvoicechat.network

import com.alaydriem.bedrockvoicechat.dto.Payload
import com.google.gson.Gson
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
        .connectTimeout(Duration.ofSeconds(1))
        .build()
) {
    companion object {
        private val GSON = Gson()
    }

    /**
     * Send player data payload asynchronously to the BVC server.
     */
    fun sendAsync(payload: Payload) {
        val jsonBody = GSON.toJson(payload)

        val request = HttpRequest.newBuilder()
            .uri(URI.create("$serverUrl/api/position"))
            .timeout(Duration.ofSeconds(1))
            .header("Content-Type", "application/json")
            .header("X-MC-Access-Token", accessToken)
            .header("Accept", "application/json")
            .POST(HttpRequest.BodyPublishers.ofString(jsonBody))
            .build()

        httpClient.sendAsync(request, HttpResponse.BodyHandlers.ofString())
            .thenAccept { 
                // Response received, no action needed
            }
            .exceptionally { 
                // Request failed, silently ignore
                null
            }
    }
}
