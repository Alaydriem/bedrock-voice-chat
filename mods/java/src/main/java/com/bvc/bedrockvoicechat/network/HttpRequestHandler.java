package com.alaydriem.bedrockvoicechat.network;

import com.alaydriem.bedrockvoicechat.config.ModConfig;
import com.alaydriem.bedrockvoicechat.dto.Payload;
import com.google.gson.Gson;

import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.time.Duration;

public class HttpRequestHandler {
    private static final Gson GSON = new Gson();

    public static void sendPlayerData(
        Payload payload,
        ModConfig config,
        HttpClient httpClient
    ) {
        String jsonBody = GSON.toJson(payload);

        HttpRequest request = HttpRequest.newBuilder()
            .uri(URI.create(config.getBvcServer() + "/api/position"))
            .timeout(Duration.ofSeconds(1))
            .header("Content-Type", "application/json")
            .header("X-MC-Access-Token", config.getAccessToken())
            .header("Accept", "application/json")
            .POST(HttpRequest.BodyPublishers.ofString(jsonBody))
            .build();

        httpClient.sendAsync(request, HttpResponse.BodyHandlers.ofString())
            .thenAccept(response -> {
            })
            .exceptionally(throwable -> {
                return null;
            });
    }
}
