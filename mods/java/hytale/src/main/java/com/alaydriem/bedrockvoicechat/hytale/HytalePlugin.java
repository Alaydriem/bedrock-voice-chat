package com.alaydriem.bedrockvoicechat.hytale;

import com.alaydriem.bedrockvoicechat.config.ModConfig;
import com.alaydriem.bedrockvoicechat.dto.Payload;
import com.alaydriem.bedrockvoicechat.dto.PlayerData;
import com.alaydriem.bedrockvoicechat.network.HttpRequestHandler;

import com.hypixel.hytale.server.core.HytaleServer;
import com.hypixel.hytale.server.core.plugin.JavaPlugin;
import com.hypixel.hytale.server.core.plugin.JavaPluginInit;
import com.hypixel.hytale.server.core.event.events.player.PlayerConnectEvent;
import com.hypixel.hytale.server.core.event.events.player.PlayerDisconnectEvent;
import com.hypixel.hytale.server.core.util.Config;

import java.util.List;
import java.util.concurrent.ScheduledFuture;
import java.util.concurrent.TimeUnit;
import java.util.logging.Level;

/**
 * Hytale plugin entry point for Bedrock Voice Chat.
 */
public class HytalePlugin extends JavaPlugin {
    private final Config<HytaleModConfig> config;
    private final HytaleConfigProvider configProvider;
    private final HytalePlayerDataProvider playerDataProvider = new HytalePlayerDataProvider();

    private ModConfig modConfig;
    private HttpRequestHandler httpHandler;
    private ScheduledFuture<?> tickTask;

    public HytalePlugin(JavaPluginInit init) {
        super(init);
        // Initialize config using Hytale's withConfig pattern
        this.config = withConfig("bedrock-voice-chat", HytaleModConfig.CODEC);
        this.configProvider = new HytaleConfigProvider(this.config);
    }

    @Override
    protected void setup() {
        getLogger().at(Level.INFO).log("Initializing Bedrock Voice Chat for Hytale");

        // Load config via provider (now backed by Hytale's Config<T>)
        modConfig = configProvider.load();
        if (!modConfig.isValid()) {
            getLogger().at(Level.SEVERE).log("Invalid configuration - plugin will not track players");
            return;
        }

        httpHandler = new HttpRequestHandler(modConfig.getBvcServer(), modConfig.getAccessToken());

        // Register player connect/disconnect events
        getEventRegistry().register(PlayerConnectEvent.class, this::onPlayerConnect);
        getEventRegistry().register(PlayerDisconnectEvent.class, this::onPlayerDisconnect);

        // Schedule tick task using Hytale's built-in executor
        // ~167ms interval = roughly 5 ticks at 30 TPS (Hytale default)
        tickTask = HytaleServer.SCHEDULED_EXECUTOR.scheduleAtFixedRate(
                this::tick, 167, 167, TimeUnit.MILLISECONDS
        );

        getLogger().at(Level.INFO).log("Bedrock Voice Chat will connect to: " + modConfig.getBvcServer());
    }

    @Override
    protected void shutdown() {
        if (tickTask != null && !tickTask.isCancelled()) {
            tickTask.cancel(false);
        }
    }

    private void onPlayerConnect(PlayerConnectEvent event) {
        playerDataProvider.addPlayer(event.getPlayerRef());
    }

    private void onPlayerDisconnect(PlayerDisconnectEvent event) {
        playerDataProvider.removePlayer(event.getPlayerRef());
    }

    private void tick() {
        try {
            List<PlayerData> players = playerDataProvider.collectPlayers();

            if (players.size() < Math.max(modConfig.getMinimumPlayers(), 2)) {
                return;
            }

            Payload payload = new Payload(playerDataProvider.getGameType(), players);
            httpHandler.sendAsync(payload);
        } catch (Exception e) {
            getLogger().at(Level.WARNING).log("Error during tick: " + e.getMessage());
        }
    }
}
