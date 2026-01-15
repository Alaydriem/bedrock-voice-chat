package com.alaydriem.bedrockvoicechat.paper;

import com.alaydriem.bedrockvoicechat.config.ModConfig;
import com.alaydriem.bedrockvoicechat.dto.Payload;
import com.alaydriem.bedrockvoicechat.dto.PlayerData;
import com.alaydriem.bedrockvoicechat.network.HttpRequestHandler;
import org.bukkit.plugin.java.JavaPlugin;
import org.bukkit.scheduler.BukkitTask;

import java.util.List;
import java.util.logging.Logger;

/**
 * Paper plugin entry point for Bedrock Voice Chat.
 */
public class PaperPlugin extends JavaPlugin {
    private final PaperConfigProvider configProvider = new PaperConfigProvider(this);
    private final PaperPlayerDataProvider playerDataProvider = new PaperPlayerDataProvider();

    private ModConfig config;
    private HttpRequestHandler httpHandler;
    private BukkitTask tickTask;

    @Override
    public void onEnable() {
        Logger logger = getLogger();
        logger.info("Initializing Bedrock Voice Chat");

        // Create default config if missing
        configProvider.createDefaultIfMissing();

        // Load and validate configuration
        config = configProvider.load();
        if (!config.isValid()) {
            logger.severe("Invalid configuration - plugin will not track players");
            logger.severe("Please configure bvc-server and access-token in config.yml");
            return;
        }

        httpHandler = new HttpRequestHandler(config.getBvcServer(), config.getAccessToken());

        // Schedule tick task every 5 ticks (250ms at 20 TPS)
        tickTask = getServer().getScheduler().runTaskTimer(this, this::tick, 0L, 5L);

        logger.info("Bedrock Voice Chat will connect to: " + config.getBvcServer());
    }

    @Override
    public void onDisable() {
        if (tickTask != null) {
            tickTask.cancel();
            tickTask = null;
        }
        getLogger().info("Bedrock Voice Chat disabled");
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
