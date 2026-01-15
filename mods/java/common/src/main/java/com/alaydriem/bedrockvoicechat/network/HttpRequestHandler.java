package com.alaydriem.bedrockvoicechat.network;

import com.alaydriem.bedrockvoicechat.dto.Payload;
import com.google.gson.Gson;

import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.time.Duration;

/**
 * Handles HTTP requests to the BVC server.
 */
public class HttpRequestHandler {
    private static final Gson GSON = new Gson();

    private final String serverUrl;
    private final String accessToken;
    private final HttpClient httpClient;

    public HttpRequestHandler(String serverUrl, String accessToken) {
        this.serverUrl = serverUrl;
        this.accessToken = accessToken;
        this.httpClient = HttpClient.newBuilder()
                .connectTimeout(Duration.ofSeconds(1))
                .build();
    }

    public HttpRequestHandler(String serverUrl, String accessToken, HttpClient httpClient) {
        this.serverUrl = serverUrl;
        this.accessToken = accessToken;
        this.httpClient = httpClient;
    }

    /**
     * Send player data payload asynchronously to the BVC server.
     */
    public void sendAsync(Payload payload) {
        String jsonBody = GSON.toJson(payload);

        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(serverUrl + "/api/position"))
                .timeout(Duration.ofSeconds(1))
                .header("Content-Type", "application/json")
                .header("X-MC-Access-Token", accessToken)
                .header("Accept", "application/json")
                .POST(HttpRequest.BodyPublishers.ofString(jsonBody))
                .build();

        httpClient.sendAsync(request, HttpResponse.BodyHandlers.ofString())
                .thenAccept(response -> {
                    // Response received, no action needed
                })
                .exceptionally(throwable -> {
                    // Request failed, silently ignore
                    return null;
                });
    }
}
