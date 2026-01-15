package com.alaydriem.bedrockvoicechat.fabric;

import com.alaydriem.bedrockvoicechat.config.ModConfig;
import com.alaydriem.bedrockvoicechat.dto.Payload;
import com.alaydriem.bedrockvoicechat.dto.PlayerData;
import com.alaydriem.bedrockvoicechat.network.HttpRequestHandler;
import net.fabricmc.api.ModInitializer;
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerTickEvents;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.util.List;

/**
 * Fabric mod entry point for Bedrock Voice Chat.
 */
public class FabricMod implements ModInitializer {
    private static final Logger LOGGER = LoggerFactory.getLogger("Bedrock Voice Chat");

    private final FabricConfigProvider configProvider = new FabricConfigProvider();
    private final FabricPlayerDataProvider playerDataProvider = new FabricPlayerDataProvider();

    private ModConfig config;
    private HttpRequestHandler httpHandler;
    private int tickCounter = 0;

    @Override
    public void onInitialize() {
        LOGGER.info("Initializing Bedrock Voice Chat");

        // Load and validate configuration
        config = configProvider.load();
        if (!config.isValid()) {
            LOGGER.error("Invalid configuration - mod will not track players");
            return;
        }

        httpHandler = new HttpRequestHandler(config.getBvcServer(), config.getAccessToken());

        ServerTickEvents.END_SERVER_TICK.register(server -> {
            playerDataProvider.setServer(server);

            tickCounter++;
            if (tickCounter >= 5) {
                tickCounter = 0;
                tick();
            }
        });

        LOGGER.info("Bedrock Voice Chat will connect to: {}", config.getBvcServer());
    }

    private void tick() {
        List<PlayerData> players = playerDataProvider.collectPlayers();

        if (players.size() < Math.max(config.getMinimumPlayers(), 2)) {
            return;
        }

        Payload payload = new Payload(playerDataProvider.getGameType(), players);
        httpHandler.sendAsync(payload);
    }
}
