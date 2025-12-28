package com.bvc.bedrockvoicechat;

import com.bvc.bedrockvoicechat.config.ConfigManager;
import com.bvc.bedrockvoicechat.config.ModConfig;
import com.bvc.bedrockvoicechat.tracker.PlayerTrackerTask;
import net.fabricmc.api.ModInitializer;
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerTickEvents;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.net.http.HttpClient;
import java.time.Duration;

public class BedrockVoiceChatMod implements ModInitializer {
    private static final Logger LOGGER = LoggerFactory.getLogger("Bedrock Voice Chat");
    private static int tickCounter = 0;
    private static ModConfig config;
    private static HttpClient httpClient;

    @Override
    public void onInitialize() {
        LOGGER.info("Initializing Bedrock Voice Chat");

        // Load and validate configuration
        config = ConfigManager.load();
        if (!config.isValid()) {
            LOGGER.error("Invalid configuration - mod will not track players");
            return;
        }

        // Initialize HTTP client (Java 11+ built-in)
        httpClient = HttpClient.newBuilder()
            .connectTimeout(Duration.ofSeconds(1))
            .build();

        // Register server tick event for 5-tick intervals
        ServerTickEvents.END_SERVER_TICK.register(server -> {
            tickCounter++;
            if (tickCounter >= 5) {
                tickCounter = 0;
                PlayerTrackerTask.execute(server, config, httpClient);
            }
        });

        LOGGER.info("Bedrock Voice Chat initialized successfully");
    }
}
