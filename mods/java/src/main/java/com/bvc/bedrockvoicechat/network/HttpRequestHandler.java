package com.bvc.bedrockvoicechat.network;

import com.bvc.bedrockvoicechat.config.ModConfig;
import com.bvc.bedrockvoicechat.dto.Player;
import com.google.gson.Gson;

import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.time.Duration;
import java.util.List;

public class HttpRequestHandler {
    private static final Gson GSON = new Gson();

    public static void sendPlayerData(
        List<Player> playerDataList,
        ModConfig config,
        HttpClient httpClient
    ) {
        // Serialize to JSON
        String jsonBody = GSON.toJson(playerDataList);

        // Build HTTP request
        HttpRequest request = HttpRequest.newBuilder()
            .uri(URI.create(config.getBvcServer() + "/api/position"))
            .timeout(Duration.ofSeconds(1))
            .header("Content-Type", "application/json")
            .header("X-MC-Access-Token", config.getAccessToken())
            .header("Accept", "application/json")
            .POST(HttpRequest.BodyPublishers.ofString(jsonBody))
            .build();

        // Send asynchronously (non-blocking)
        httpClient.sendAsync(request, HttpResponse.BodyHandlers.ofString())
            .thenAccept(response -> {
                // Success - silent (no logging)
            })
            .exceptionally(throwable -> {
                // Error - silent (matches BDS behavior)
                return null;
            });
    }
}
